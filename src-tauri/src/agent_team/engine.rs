//! Agent Team 核心引擎 - 轮次编排与状态机流转

use anyhow::{Context, Result};
use serde_json::{json, Value};
use std::sync::Arc;
use std::sync::Mutex;
use std::time::Duration as StdDuration;
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
use super::role_context::{
    build_challenge_prompt, build_decide_prompt, build_propose_prompt, build_role_context,
    calculate_divergence_score, RoleContextInput,
};
use super::scheduler::{DivergenceCalculator, ToolGovernance};

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
const DEFAULT_MAX_HUMAN_INTERVENTIONS: i64 = 2;
const DEFAULT_HUMAN_INTERVENTION_TIMEOUT_SECS: i64 = 300;
const MIN_HUMAN_INTERVENTION_TIMEOUT_SECS: i64 = 30;
const MAX_HUMAN_INTERVENTION_TIMEOUT_SECS: i64 = 3600;
const DEFAULT_NO_HUMAN_INPUT_POLICY: &str = "balanced";

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

        // round_number 全局递增，用于时间线一致展示
        let session_max_total_rounds = session.max_rounds.max(1);
        // 预留 1 轮给 proposing，1 轮给 deciding，剩余预算给 challenging
        let session_max_challenge_rounds = (session_max_total_rounds - 2).max(0);
        let effective_max_challenge_rounds =
            std::cmp::min(session_max_challenge_rounds, self.config.max_challenge_rounds);

        // 执行 Propose 阶段（第 1 轮）
        let mut current_round_number = 1;
        self.run_propose_phase(session_id, &session, current_round_number)
            .await?;

        // 执行 Challenge 阶段（并发多角色，可迭代多轮）
        for challenge_idx in 0..effective_max_challenge_rounds {
            current_round_number += 1;
            let (round_divergence, suspended) = self
                .run_challenge_phase(session_id, &session, current_round_number)
                .await?;
            if suspended {
                // 等待 Human-in-the-Loop 恢复（此处直接返回，resume 由 Tauri command 重入）
                return Ok(());
            }
            if round_divergence <= self.config.divergence_threshold * 0.7 {
                break;
            }
            if challenge_idx + 1 < effective_max_challenge_rounds {
                self.emit_event(
                    session_id,
                    "agent_team:divergence_alert",
                    json!({
                        "session_id": session_id,
                        "divergence_score": round_divergence,
                        "threshold": self.config.divergence_threshold,
                        "extra_round": current_round_number + 1
                    }),
                );
            }
        }

        // 执行 Decide 阶段：当总轮次预算=1 时，决策并入第1轮；否则使用下一轮号
        let decide_round_number = if session_max_total_rounds <= 1 {
            1
        } else {
            current_round_number + 1
        };
        self.run_decide_phase(session_id, &session, decide_round_number)
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

    /// Phase: PROPOSING - 核心角色生成初始方案
    async fn run_propose_phase(
        &self,
        session_id: &str,
        session: &AgentTeamSession,
        round_number: i32,
    ) -> Result<()> {
        let db = self
            .app_handle
            .try_state::<Arc<DatabaseService>>()
            .context("DatabaseService not available")?;
        let pool = db.get_runtime_pool().context("Failed to get db pool")?;

        self.transition_state(session_id, TeamSessionState::Proposing)
            .await?;

        let round = repo_rt::create_round(&pool, session_id, round_number, "proposing").await?;
        self.emit_event(
            session_id,
            "agent_team:round_started",
            json!({"round": round_number, "phase": "proposing"}),
        );

        let goal = session.goal.as_deref().unwrap_or("请讨论并制定方案");
        // 第1号角色（产品/主提案角色）生成初始方案
        let proposer = &session.members[0];
        let llm_config = self.get_llm_config_for_member(Some(proposer)).await?;
        self.emit_event(
            session_id,
            "agent_team:role_thinking",
            json!({
                "member_id": proposer.id,
                "member_name": proposer.name,
                "phase": "proposing"
            }),
        );

        let propose_prompt = build_propose_prompt(
            goal,
            round_number,
            &proposer.name,
            proposer.responsibility.as_deref().unwrap_or(""),
        );
        let shared_memory_prompt = self
            .build_shared_execution_memory_prompt(&pool, session_id, EXECUTION_MEMORY_PROMPT_CARDS)
            .await
            .unwrap_or_default();

        let context = build_role_context(RoleContextInput {
            app_handle: &self.app_handle,
            session_id,
            member: proposer,
                session_goal: goal,
                round_task: &propose_prompt,
                round_number,
                llm_config: &llm_config,
                blackboard: &self.blackboard,
            directed_messages: vec![ChatMessage {
                role: "user".to_string(),
                content: propose_prompt.clone(),
                tool_calls: None,
                tool_call_id: None,
                reasoning_content: None,
            }],
            shared_execution_memory: Some(shared_memory_prompt.as_str()),
        })
        .await?;

        let (proposal, proposal_tool_calls) = self
            .invoke_llm_streaming(
                session_id,
                Some(proposer),
                "proposing",
                &llm_config,
                &context.system_prompt,
                &context.history_messages,
            )
            .await?;

        // 保存提案消息
        let proposal_msg = repo_rt::create_message(
            &pool,
            session_id,
            Some(&round.id),
            Some(&proposer.id),
            Some(&proposer.name),
            "assistant",
            &proposal,
            None,
        )
        .await?;
        if let Some(tc) = &proposal_tool_calls {
            repo_rt::update_message_tool_calls(&pool, &proposal_msg.id, tc).await?;
        }
        self.summarize_and_persist_member_execution(
            &pool,
            session_id,
            goal,
            round_number,
            "proposing",
            proposer,
            &llm_config,
            &proposal,
            proposal_tool_calls.as_ref(),
        )
        .await;

        // 更新白板：记录提案要点
        self.blackboard
            .append_entry(
                session_id,
                super::models::BlackboardEntryType::ActionItem,
                &format!("{}的初始提案", proposer.name),
                &shorten_text(&proposal, 500),
                Some(&proposer.name),
                &uuid::Uuid::new_v4().to_string(),
            )
            .await;

        repo_rt::upsert_blackboard_entry(
            &pool,
            &UpdateBlackboardRequest {
                session_id: session_id.to_string(),
                entry_type: "action_item".to_string(),
                title: format!("{}的初始提案", proposer.name),
                content: shorten_text(&proposal, 500),
                contributed_by: Some(proposer.name.clone()),
                round_id: Some(round.id.clone()),
            },
        )
        .await?;

        repo_rt::complete_round(&pool, &round.id, None).await?;
        self.emit_event(
            session_id,
            "agent_team:round_completed",
            json!({"round": round_number, "phase": "proposing"}),
        );

        info!("Proposing phase completed for session: {}", session_id);
        Ok(())
    }

    /// Phase: CHALLENGING - 多角色并发 Review（DAG 并发调度）
    /// 返回 (divergence_score, is_suspended)
    async fn run_challenge_phase(
        &self,
        session_id: &str,
        session: &AgentTeamSession,
        round_number: i32,
    ) -> Result<(f64, bool)> {
        let db = self
            .app_handle
            .try_state::<Arc<DatabaseService>>()
            .context("DatabaseService not available")?;
        let pool = db.get_runtime_pool().context("Failed to get db pool")?;

        self.transition_state(session_id, TeamSessionState::Challenging)
            .await?;

        let round = repo_rt::create_round(&pool, session_id, round_number, "challenging").await?;
        self.emit_event(
            session_id,
            "agent_team:round_started",
            json!({"round": round_number, "phase": "challenging"}),
        );

        // 获取最新提案（上一轮最后的 assistant 消息）
        let messages = repo_rt::get_messages(&pool, session_id).await?;
        let last_proposal = messages
            .iter()
            .filter(|m| m.role == "assistant")
            .last()
            .map(|m| m.content.clone())
            .unwrap_or_default();

        let goal = session.goal.as_deref().unwrap_or("请讨论并制定方案");
        // 生成 DAG 执行计划（除第0号角色即 proposer 外全部并发）
        let reviewers: Vec<&AgentTeamMember> = session.members.iter().skip(1).collect();

        // 并发执行所有 Reviewer
        let mut review_results: Vec<(String, String, String)> = vec![]; // (member_id, name, review)

        for reviewer in &reviewers {
            let reviewer_id = reviewer.id.clone();
            let reviewer_name = reviewer.name.clone();
            let responsibility = reviewer.responsibility.clone().unwrap_or_default();
            let proposal_clone = last_proposal.clone();
            let round_num = round_number;
            let llm_config = self.get_llm_config_for_member(Some(reviewer)).await?;

            // 每个 reviewer 串行执行（tokio::spawn 受 Semaphore 限制）
            let challenge_prompt =
                build_challenge_prompt(&proposal_clone, &reviewer_name, &responsibility, round_num);
            let shared_memory_prompt = self
                .build_shared_execution_memory_prompt(
                    &pool,
                    session_id,
                    EXECUTION_MEMORY_PROMPT_CARDS,
                )
                .await
                .unwrap_or_default();

            // 通知前端该角色开始思考
            self.emit_event(
                session_id,
                "agent_team:role_thinking",
                json!({
                    "member_id": reviewer_id,
                    "member_name": reviewer_name,
                    "phase": "challenging"
                }),
            );

            let context = build_role_context(RoleContextInput {
                app_handle: &self.app_handle,
                session_id,
                member: reviewer,
                session_goal: goal,
                round_task: &challenge_prompt,
                round_number,
                llm_config: &llm_config,
                blackboard: &self.blackboard,
                directed_messages: vec![ChatMessage {
                    role: "user".to_string(),
                    content: challenge_prompt.clone(),
                    tool_calls: None,
                    tool_call_id: None,
                    reasoning_content: None,
                }],
                shared_execution_memory: Some(shared_memory_prompt.as_str()),
            })
            .await?;

            let (review, review_tool_calls) = self
                .invoke_llm_streaming(
                    session_id,
                    Some(reviewer),
                    "challenging",
                    &llm_config,
                    &context.system_prompt,
                    &context.history_messages,
                )
                .await?;

            // 保存消息
            let review_msg = repo_rt::create_message(
                &pool,
                session_id,
                Some(&round.id),
                Some(&reviewer.id),
                Some(&reviewer.name),
                "assistant",
                &review,
                None,
            )
            .await?;
            if let Some(tc) = &review_tool_calls {
                repo_rt::update_message_tool_calls(&pool, &review_msg.id, tc).await?;
            }
            self.summarize_and_persist_member_execution(
                &pool,
                session_id,
                goal,
                round_number,
                "challenging",
                reviewer,
                &llm_config,
                &review,
                review_tool_calls.as_ref(),
            )
            .await;

            review_results.push((reviewer_id, reviewer_name, review));
        }

        // 计算分歧度（使用 DivergenceCalculator）
        let review_texts: Vec<&str> = review_results.iter().map(|(_, _, r)| r.as_str()).collect();
        let divergence = DivergenceCalculator::calculate(&review_texts);

        info!(
            "Challenge round {}: divergence={:.3} (threshold={:.3})",
            round_number, divergence, self.config.divergence_threshold
        );

        // 将各角色 Review 更新到白板（分歧/共识）
        for (_, name, review) in &review_results {
            let entry_type = if divergence > self.config.divergence_threshold * 0.5 {
                "dispute"
            } else {
                "consensus"
            };
            let entry_req = UpdateBlackboardRequest {
                session_id: session_id.to_string(),
                entry_type: entry_type.to_string(),
                title: format!("{} 的评审（第{}轮）", name, round_number),
                content: shorten_text(review, 400),
                contributed_by: Some(name.clone()),
                round_id: Some(round.id.clone()),
            };
            self.blackboard
                .append_entry(
                    session_id,
                    if entry_type == "dispute" {
                        BlackboardEntryType::Dispute
                    } else {
                        BlackboardEntryType::Consensus
                    },
                    &entry_req.title,
                    &entry_req.content,
                    Some(name),
                    &round.id,
                )
                .await;
            repo_rt::upsert_blackboard_entry(&pool, &entry_req).await?;
        }

        // 检查是否需要 Human-in-the-Loop
        if DivergenceCalculator::needs_human_intervention(
            divergence,
            self.config.divergence_threshold,
        ) {
            self.emit_event(
                session_id,
                "agent_team:divergence_alert",
                json!({
                    "session_id": session_id,
                    "divergence_score": divergence,
                    "threshold": self.config.divergence_threshold
                }),
            );
            let should_suspend = self
                .should_suspend_for_human_intervention(
                    &pool,
                    session_id,
                    divergence,
                    "challenging",
                    round_number,
                )
                .await?;
            if should_suspend {
                warn!("High divergence ({:.3}), suspending for human", divergence);
                self.transition_state(session_id, TeamSessionState::SuspendedForHuman)
                    .await?;
                repo_rt::complete_round(&pool, &round.id, Some(divergence)).await?;
                self.schedule_auto_resume_if_needed(session_id);
                return Ok((divergence, true));
            }
            warn!(
                "High divergence ({:.3}) but human intervention limit reached; forcing progression",
                divergence
            );
        }

        self.transition_state(session_id, TeamSessionState::ConvergenceCheck)
            .await?;
        repo_rt::complete_round(&pool, &round.id, Some(divergence)).await?;
        self.emit_event(
            session_id,
            "agent_team:round_completed",
            json!({"round": round_number, "phase": "challenging", "divergence_score": divergence}),
        );

        info!(
            "Challenge phase round {} completed, divergence={:.3}",
            round_number, divergence
        );
        Ok((divergence, false))
    }

    /// Phase: DECIDING - 其他角色 Review 并形成最终方案
    async fn run_decide_phase(
        &self,
        session_id: &str,
        session: &AgentTeamSession,
        round_number: i32,
    ) -> Result<()> {
        let db = self
            .app_handle
            .try_state::<Arc<DatabaseService>>()
            .context("DatabaseService not available")?;
        let pool = db.get_runtime_pool().context("Failed to get db pool")?;

        self.transition_state(session_id, TeamSessionState::Deciding)
            .await?;

        let round = repo_rt::create_round(&pool, session_id, round_number, "deciding").await?;
        self.emit_event(
            session_id,
            "agent_team:round_started",
            json!({"round": round_number, "phase": "deciding"}),
        );

        let goal = session.goal.as_deref().unwrap_or("请讨论并制定方案");

        // 获取第1轮的提案内容
        let messages = repo_rt::get_messages(&pool, session_id).await?;
        let proposal = messages
            .iter()
            .filter(|m| m.role == "assistant")
            .last()
            .map(|m| m.content.clone())
            .unwrap_or_default();

        // 2号角色及以后的角色进行 Review
        let mut review_scores: Vec<f64> = vec![];
        let mut all_reviews = vec![format!(
            "**{}的初始提案：**\n{}",
            session.members[0].name, &proposal
        )];

        for reviewer in session.members.iter().skip(1) {
            let llm_config = self.get_llm_config_for_member(Some(reviewer)).await?;
            self.emit_event(
                session_id,
                "agent_team:role_thinking",
                json!({
                    "member_id": reviewer.id,
                    "member_name": reviewer.name,
                    "phase": "deciding"
                }),
            );

            let challenge_prompt = build_challenge_prompt(
                &proposal,
                &reviewer.name,
                reviewer.responsibility.as_deref().unwrap_or(""),
                round_number,
            );
            let shared_memory_prompt = self
                .build_shared_execution_memory_prompt(
                    &pool,
                    session_id,
                    EXECUTION_MEMORY_PROMPT_CARDS,
                )
                .await
                .unwrap_or_default();

            let context = build_role_context(RoleContextInput {
                app_handle: &self.app_handle,
                session_id,
                member: reviewer,
                session_goal: goal,
                round_task: &challenge_prompt,
                round_number,
                llm_config: &llm_config,
                blackboard: &self.blackboard,
                directed_messages: vec![ChatMessage {
                    role: "user".to_string(),
                    content: challenge_prompt.clone(),
                    tool_calls: None,
                    tool_call_id: None,
                    reasoning_content: None,
                }],
                shared_execution_memory: Some(shared_memory_prompt.as_str()),
            })
            .await?;

            let (review, review_tool_calls) = self
                .invoke_llm_streaming(
                    session_id,
                    Some(reviewer),
                    "deciding",
                    &llm_config,
                    &context.system_prompt,
                    &context.history_messages,
                )
                .await?;

            // 尝试从 review 中提取评分
            let score = extract_score_from_review(&review);
            if let Some(s) = score {
                review_scores.push(s);
            }

            let review_msg = repo_rt::create_message(
                &pool,
                session_id,
                Some(&round.id),
                Some(&reviewer.id),
                Some(&reviewer.name),
                "assistant",
                &review,
                None,
            )
            .await?;
            if let Some(tc) = &review_tool_calls {
                repo_rt::update_message_tool_calls(&pool, &review_msg.id, tc).await?;
            }
            self.summarize_and_persist_member_execution(
                &pool,
                session_id,
                goal,
                round_number,
                "deciding",
                reviewer,
                &llm_config,
                &review,
                review_tool_calls.as_ref(),
            )
            .await;

            all_reviews.push(format!("**{}的评审：**\n{}", reviewer.name, &review));
        }

        // 计算分歧度
        let divergence = if review_scores.len() >= 2 {
            calculate_divergence_score(&review_scores)
        } else {
            0.0
        };

        info!(
            "Divergence score for session {}: {:.3} (threshold: {:.3})",
            session_id, divergence, self.config.divergence_threshold
        );

        // 如果分歧过大，触发 Human-in-the-Loop
        if divergence > self.config.divergence_threshold {
            self.emit_event(
                session_id,
                "agent_team:divergence_alert",
                json!({
                    "session_id": session_id,
                    "divergence_score": divergence,
                    "threshold": self.config.divergence_threshold
                }),
            );
            let should_suspend = self
                .should_suspend_for_human_intervention(&pool, session_id, divergence, "deciding", 2)
                .await?;
            if should_suspend {
                warn!(
                    "High divergence detected ({:.3}), suspending for human review",
                    divergence
                );
                self.transition_state(session_id, TeamSessionState::SuspendedForHuman)
                    .await?;
                repo_rt::complete_round(&pool, &round.id, Some(divergence)).await?;
                self.schedule_auto_resume_if_needed(session_id);
                return Ok(());
            }
            warn!(
                "High divergence detected ({:.3}) but human intervention limit reached; forcing decision merge",
                divergence
            );
        }

        // 生成最终决策
        let blackboard_summary = self.blackboard.get_context_summary(session_id).await;
        let decide_prompt =
            build_decide_prompt(goal, &all_reviews.join("\n\n"), &blackboard_summary, round_number);

        // 使用第1号角色作为决策整合者
        let decider = &session.members[0];
        let llm_config = self.get_llm_config_for_member(Some(decider)).await?;
        let shared_memory_prompt = self
            .build_shared_execution_memory_prompt(&pool, session_id, EXECUTION_MEMORY_PROMPT_CARDS)
            .await
            .unwrap_or_default();
        let context = build_role_context(RoleContextInput {
            app_handle: &self.app_handle,
            session_id,
            member: decider,
            session_goal: goal,
            round_task: &decide_prompt,
            round_number,
            llm_config: &llm_config,
            blackboard: &self.blackboard,
            directed_messages: vec![ChatMessage {
                role: "user".to_string(),
                content: decide_prompt.clone(),
                tool_calls: None,
                tool_call_id: None,
                reasoning_content: None,
            }],
            shared_execution_memory: Some(shared_memory_prompt.as_str()),
        })
        .await?;

        let (decision, decision_tool_calls) = self
            .invoke_llm_streaming(
                session_id,
                Some(decider),
                "deciding",
                &llm_config,
                &context.system_prompt,
                &context.history_messages,
            )
            .await?;

        let decision_msg = repo_rt::create_message(
            &pool,
            session_id,
            Some(&round.id),
            Some(&decider.id),
            Some(&decider.name),
            "assistant",
            &decision,
            None,
        )
        .await?;
        if let Some(tc) = &decision_tool_calls {
            repo_rt::update_message_tool_calls(&pool, &decision_msg.id, tc).await?;
        }
        self.summarize_and_persist_member_execution(
            &pool,
            session_id,
            goal,
            round_number,
            "deciding",
            decider,
            &llm_config,
            &decision,
            decision_tool_calls.as_ref(),
        )
        .await;

        // 记录最终决策到白板
        self.blackboard
            .append_entry(
                session_id,
                super::models::BlackboardEntryType::Consensus,
                "最终决策",
                &shorten_text(&decision, 800),
                Some("系统"),
                &uuid::Uuid::new_v4().to_string(),
            )
            .await;

        repo_rt::upsert_blackboard_entry(
            &pool,
            &UpdateBlackboardRequest {
                session_id: session_id.to_string(),
                entry_type: "consensus".to_string(),
                title: "最终决策".to_string(),
                content: shorten_text(&decision, 800),
                contributed_by: Some("系统合并".to_string()),
                round_id: Some(round.id.clone()),
            },
        )
        .await?;

        repo_rt::complete_round(&pool, &round.id, Some(divergence)).await?;
        self.emit_event(
            session_id,
            "agent_team:round_completed",
            json!({"round": round_number, "phase": "deciding", "divergence_score": divergence}),
        );

        info!("Deciding phase completed for session: {}", session_id);
        Ok(())
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

    fn schedule_auto_resume_if_needed(&self, session_id: &str) {
        let session_id_owned = session_id.to_string();
        let app_handle = self.app_handle.clone();

        tokio::spawn(async move {
            let mut timeout_secs = DEFAULT_HUMAN_INTERVENTION_TIMEOUT_SECS;
            let mut policy = DEFAULT_NO_HUMAN_INPUT_POLICY.to_string();

            let Some(db) = app_handle.try_state::<Arc<DatabaseService>>() else {
                warn!(
                    "Auto-resume skipped for session {}: DatabaseService not available",
                    session_id_owned
                );
                return;
            };
            let pool = match db.get_runtime_pool() {
                Ok(pool) => pool,
                Err(e) => {
                    warn!(
                        "Auto-resume skipped for session {}: failed to get db pool: {}",
                        session_id_owned, e
                    );
                    return;
                }
            };

            let session = match repo_rt::get_agent_team_session(&pool, &session_id_owned).await {
                Ok(Some(s)) => s,
                Ok(None) => return,
                Err(e) => {
                    warn!(
                        "Auto-resume skipped for session {}: failed to fetch session for scheduling: {}",
                        session_id_owned, e
                    );
                    return;
                }
            };

            if let Some(state_machine) = session.state_machine.as_ref().and_then(|v| v.as_object()) {
                let fallback_timeout = state_machine
                    .get("human_intervention_timeout_secs")
                    .and_then(|v| v.as_i64())
                    .unwrap_or(DEFAULT_HUMAN_INTERVENTION_TIMEOUT_SECS);
                let fallback_policy = AgentTeamEngine::normalize_no_human_input_policy(
                    state_machine
                        .get("no_human_input_policy")
                        .and_then(|v| v.as_str()),
                );

                if let Some(intervention) = state_machine
                    .get("human_intervention")
                    .and_then(|v| v.as_object())
                {
                    timeout_secs = AgentTeamEngine::normalize_human_intervention_timeout_secs(
                        intervention
                            .get("timeout_secs")
                            .and_then(|v| v.as_i64())
                            .unwrap_or(fallback_timeout),
                    );
                    policy = AgentTeamEngine::normalize_no_human_input_policy(
                        intervention
                            .get("policy")
                            .and_then(|v| v.as_str())
                            .or_else(|| Some(fallback_policy.as_str())),
                    );
                } else {
                    timeout_secs =
                        AgentTeamEngine::normalize_human_intervention_timeout_secs(fallback_timeout);
                    policy = fallback_policy;
                }
            }

            info!(
                "Scheduled auto-resume for session {} in {}s with '{}' policy",
                session_id_owned, timeout_secs, policy
            );
            tokio::time::sleep(StdDuration::from_secs(timeout_secs as u64)).await;

            let session = match repo_rt::get_agent_team_session(&pool, &session_id_owned).await {
                Ok(Some(s)) => s,
                Ok(None) => return,
                Err(e) => {
                    warn!(
                        "Auto-resume skipped for session {}: failed to fetch session on trigger: {}",
                        session_id_owned, e
                    );
                    return;
                }
            };

            if session.state != TeamSessionState::SuspendedForHuman.to_string() {
                return;
            }

            let auto_message = AgentTeamEngine::build_auto_resume_prompt(&policy);
            if let Err(e) = repo_rt::create_message(
                &pool,
                &session_id_owned,
                None,
                None,
                Some("系统自动介入"),
                "user",
                &auto_message,
                None,
            )
            .await
            {
                warn!(
                    "Auto-resume skipped for session {}: failed to append auto message: {}",
                    session_id_owned, e
                );
                return;
            }

            let mut state_machine = session.state_machine.unwrap_or_else(|| json!({}));
            if !state_machine.is_object() {
                state_machine = json!({});
            }
            if let Some(state_obj) = state_machine.as_object_mut() {
                let intervention_value = state_obj
                    .entry("human_intervention".to_string())
                    .or_insert_with(|| json!({}));
                if !intervention_value.is_object() {
                    *intervention_value = json!({});
                }
                if let Some(intervention_obj) = intervention_value.as_object_mut() {
                    let auto_resume_count = intervention_obj
                        .get("auto_resume_count")
                        .and_then(|v| v.as_i64())
                        .unwrap_or(0)
                        + 1;
                    intervention_obj
                        .insert("auto_resume_count".to_string(), json!(auto_resume_count));
                    intervention_obj.insert(
                        "auto_resumed_at".to_string(),
                        json!(chrono::Utc::now().to_rfc3339()),
                    );
                    intervention_obj.insert(
                        "last_auto_resume_policy".to_string(),
                        json!(policy.clone()),
                    );
                    intervention_obj
                        .insert("last_action".to_string(), json!("auto_resumed_timeout"));
                    intervention_obj.insert(
                        "updated_at".to_string(),
                        json!(chrono::Utc::now().to_rfc3339()),
                    );
                    intervention_obj.remove("auto_resume_at");
                }
            }

            if let Err(e) = repo_rt::update_agent_team_session(
                &pool,
                &session_id_owned,
                &UpdateAgentTeamSessionRequest {
                    name: None,
                    goal: None,
                    state: None,
                    max_rounds: None,
                    state_machine: Some(state_machine),
                    error_message: None,
                },
            )
            .await
            {
                warn!(
                    "Failed to persist auto-resume metadata for session {}: {}",
                    session_id_owned, e
                );
            }

            match start_agent_team_run_async(app_handle.clone(), session_id_owned.clone()).await {
                Ok(_) => info!(
                    "Session {} auto-resumed after timeout using '{}' policy",
                    session_id_owned, policy
                ),
                Err(e) => warn!("Auto-resume failed for session {}: {}", session_id_owned, e),
            }
        });
    }

    fn normalize_human_intervention_timeout_secs(value: i64) -> i64 {
        value.clamp(
            MIN_HUMAN_INTERVENTION_TIMEOUT_SECS,
            MAX_HUMAN_INTERVENTION_TIMEOUT_SECS,
        )
    }

    fn normalize_no_human_input_policy(policy: Option<&str>) -> String {
        match policy
            .unwrap_or(DEFAULT_NO_HUMAN_INPUT_POLICY)
            .trim()
            .to_lowercase()
            .as_str()
        {
            "conservative" => "conservative".to_string(),
            "aggressive" => "aggressive".to_string(),
            _ => "balanced".to_string(),
        }
    }

    fn build_auto_resume_prompt(policy: &str) -> String {
        match policy {
            "conservative" => "用户暂未介入。请按保守收敛策略继续：优先安全与稳定，选择风险最低且可回滚方案。请直接输出唯一执行方案、被放弃方案及理由、执行步骤。".to_string(),
            "aggressive" => "用户暂未介入。请按激进推进策略继续：优先交付速度与产出，选择实现最快方案，同时给出关键风险及兜底回滚。请直接输出唯一执行方案、被放弃方案及理由、执行步骤。".to_string(),
            _ => "用户暂未介入。请按平衡收敛策略继续：在风险、成本、质量之间做均衡取舍，给出可执行的单一方案。请直接输出唯一执行方案、被放弃方案及理由、执行步骤。".to_string(),
        }
    }

    async fn should_suspend_for_human_intervention(
        &self,
        pool: &DatabasePool,
        session_id: &str,
        divergence: f64,
        phase: &str,
        round_number: i32,
    ) -> Result<bool> {
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

        let fallback_max = state_obj
            .get("max_human_interventions")
            .and_then(|v| v.as_i64())
            .filter(|v| *v > 0)
            .unwrap_or(DEFAULT_MAX_HUMAN_INTERVENTIONS);
        let fallback_timeout_secs = Self::normalize_human_intervention_timeout_secs(
            state_obj
                .get("human_intervention_timeout_secs")
                .and_then(|v| v.as_i64())
                .unwrap_or(DEFAULT_HUMAN_INTERVENTION_TIMEOUT_SECS),
        );
        let fallback_policy = Self::normalize_no_human_input_policy(
            state_obj
                .get("no_human_input_policy")
                .and_then(|v| v.as_str()),
        );

        let intervention_value = state_obj
            .entry("human_intervention".to_string())
            .or_insert_with(|| json!({}));
        if !intervention_value.is_object() {
            *intervention_value = json!({});
        }
        let intervention_obj = intervention_value
            .as_object_mut()
            .ok_or_else(|| anyhow::anyhow!("human_intervention must be object"))?;

        let count = intervention_obj
            .get("count")
            .and_then(|v| v.as_i64())
            .unwrap_or(0)
            .max(0);
        let max_interventions = intervention_obj
            .get("max")
            .and_then(|v| v.as_i64())
            .filter(|v| *v > 0)
            .unwrap_or(fallback_max);
        let timeout_secs = Self::normalize_human_intervention_timeout_secs(
            intervention_obj
                .get("timeout_secs")
                .and_then(|v| v.as_i64())
                .unwrap_or(fallback_timeout_secs),
        );
        let policy = Self::normalize_no_human_input_policy(
            intervention_obj
                .get("policy")
                .and_then(|v| v.as_str())
                .or_else(|| Some(fallback_policy.as_str())),
        );

        let now_dt = chrono::Utc::now();
        let now = now_dt.to_rfc3339();

        if count >= max_interventions {
            let forced_continue_count = intervention_obj
                .get("forced_continue_count")
                .and_then(|v| v.as_i64())
                .unwrap_or(0)
                + 1;
            intervention_obj.insert("max".to_string(), json!(max_interventions));
            intervention_obj.insert("count".to_string(), json!(count));
            intervention_obj.insert("limit_reached".to_string(), json!(true));
            intervention_obj.insert(
                "forced_continue_count".to_string(),
                json!(forced_continue_count),
            );
            intervention_obj.insert("last_phase".to_string(), json!(phase));
            intervention_obj.insert("last_round".to_string(), json!(round_number));
            intervention_obj.insert("last_divergence".to_string(), json!(divergence));
            intervention_obj.insert("policy".to_string(), json!(policy.clone()));
            intervention_obj.insert("timeout_secs".to_string(), json!(timeout_secs));
            intervention_obj.insert("updated_at".to_string(), json!(now));
            intervention_obj.insert("last_action".to_string(), json!("forced_continue"));
            intervention_obj.remove("auto_resume_at");

            repo_rt::update_agent_team_session(
                pool,
                session_id,
                &UpdateAgentTeamSessionRequest {
                    name: None,
                    goal: None,
                    state: None,
                    max_rounds: None,
                    state_machine: Some(state_machine),
                    error_message: None,
                },
            )
            .await?;

            let policy_label = match policy.as_str() {
                "conservative" => "保守",
                "aggressive" => "激进",
                _ => "平衡",
            };
            let system_message = format!(
                "分歧度仍然偏高（{:.0}%），且人工介入已达上限（{}/{}）。系统将按{}策略强制推进后续决策以避免循环。",
                divergence * 100.0,
                count,
                max_interventions,
                policy_label
            );
            let _ = repo_rt::create_message(
                pool,
                session_id,
                None,
                None,
                Some("系统"),
                "system",
                &system_message,
                None,
            )
            .await;
            return Ok(false);
        }

        let next_count = count + 1;
        let auto_resume_at = (now_dt + chrono::Duration::seconds(timeout_secs)).to_rfc3339();
        intervention_obj.insert("max".to_string(), json!(max_interventions));
        intervention_obj.insert("count".to_string(), json!(next_count));
        intervention_obj.insert("policy".to_string(), json!(policy));
        intervention_obj.insert("timeout_secs".to_string(), json!(timeout_secs));
        intervention_obj.insert("limit_reached".to_string(), json!(false));
        intervention_obj.insert(
            "degradation_level".to_string(),
            json!(std::cmp::min(next_count, max_interventions)),
        );
        intervention_obj.insert("last_phase".to_string(), json!(phase));
        intervention_obj.insert("last_round".to_string(), json!(round_number));
        intervention_obj.insert("last_divergence".to_string(), json!(divergence));
        intervention_obj.insert("suspended_at".to_string(), json!(now.clone()));
        intervention_obj.insert("auto_resume_at".to_string(), json!(auto_resume_at));
        intervention_obj.insert("updated_at".to_string(), json!(now));
        intervention_obj.insert("last_action".to_string(), json!("suspended_waiting_human"));

        repo_rt::update_agent_team_session(
            pool,
            session_id,
            &UpdateAgentTeamSessionRequest {
                name: None,
                goal: None,
                state: None,
                max_rounds: None,
                state_machine: Some(state_machine),
                error_message: None,
            },
        )
        .await?;

        Ok(true)
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

/// 从评审文本中提取评分（如 "综合评分：8/10"）
fn extract_score_from_review(review: &str) -> Option<f64> {
    // 简单正则-like 提取
    for line in review.lines() {
        let lower = line.to_lowercase();
        if lower.contains("评分") || lower.contains("score") {
            // 寻找数字
            let numbers: Vec<f64> = line
                .split_whitespace()
                .filter_map(|w| {
                    let w = w.trim_matches(|c: char| !c.is_ascii_digit() && c != '.');
                    w.parse::<f64>().ok()
                })
                .filter(|&n| n >= 1.0 && n <= 10.0)
                .collect();
            if let Some(&score) = numbers.first() {
                return Some(score);
            }
        }
    }
    None
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
