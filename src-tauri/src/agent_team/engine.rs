//! Agent Team 核心引擎 - 轮次编排与状态机流转

use anyhow::{Context, Result};
use serde_json::json;
use std::sync::Mutex;
use std::sync::Arc;
use tauri::{AppHandle, Emitter, Manager};
use tracing::{error, info, warn};

use sentinel_llm::{ChatMessage, LlmConfig, StreamContent, StreamingLlmClient};
use sentinel_db::DatabaseService;
use sentinel_tools::{buildin_tools::SkillsTool, get_tool_server, DynamicTool};

use super::blackboard::BlackboardManager;
use super::models::*;
use super::repository_runtime as repo_rt;
use super::role_context::{
    build_challenge_prompt, build_decide_prompt, build_propose_prompt, build_role_context,
    calculate_divergence_score, RoleContextInput,
};
use super::scheduler::{DivergenceCalculator, SchedulePlan, ToolGovernance, run_concurrent_layer};

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
        self.emit_event(session_id, "agent_team:start", json!({"session_id": session_id}));

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
            for member in &session.members {
                if let Some(policy) = &member.tool_policy {
                    tool_governance.load_member_policy(&member.id, policy);
                }
            }
        }

        // 执行 Propose 阶段
        self.run_propose_phase(session_id, &session).await?;

        // 执行 Challenge 阶段（并发多角色，可迭代多轮）
        let mut divergence = 0.0;
        let mut challenge_round = 1;
        loop {
            let (round_divergence, suspended) = self
                .run_challenge_phase(session_id, &session, challenge_round + 1)
                .await?;
            divergence = round_divergence;
            if suspended {
                // 等待 Human-in-the-Loop 恢复（此处直接返回，resume 由 Tauri command 重入）
                return Ok(());
            }
            if divergence <= self.config.divergence_threshold * 0.7
                || challenge_round >= self.config.max_challenge_rounds
            {
                break;
            }
            challenge_round += 1;
            self.emit_event(
                session_id,
                "agent_team:divergence_alert",
                json!({
                    "session_id": session_id,
                    "divergence_score": divergence,
                    "threshold": self.config.divergence_threshold,
                    "extra_round": challenge_round
                }),
            );
        }

        // 执行 Decide 阶段
        self.run_decide_phase(session_id, &session).await?;

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
    async fn run_propose_phase(&self, session_id: &str, session: &AgentTeamSession) -> Result<()> {
        let db = self
            .app_handle
            .try_state::<Arc<DatabaseService>>()
            .context("DatabaseService not available")?;
        let pool = db.get_runtime_pool().context("Failed to get db pool")?;

        self.transition_state(session_id, TeamSessionState::Proposing)
            .await?;

        let round = repo_rt::create_round(&pool, session_id, 1, "proposing").await?;
        self.emit_event(
            session_id,
            "agent_team:round_started",
            json!({"round": 1, "phase": "proposing"}),
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
            1,
            &proposer.name,
            proposer.responsibility.as_deref().unwrap_or(""),
        );

        let context = build_role_context(RoleContextInput {
            app_handle: &self.app_handle,
            session_id,
            member: proposer,
            session_goal: goal,
            round_task: &propose_prompt,
            round_number: 1,
            llm_config: &llm_config,
            blackboard: &self.blackboard,
            directed_messages: vec![ChatMessage {
                role: "user".to_string(),
                content: propose_prompt.clone(),
                tool_calls: None,
                tool_call_id: None,
                reasoning_content: None,
            }],
        })
        .await?;

        let proposal = self
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
        repo_rt::create_message(
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
            json!({"round": 1, "phase": "proposing"}),
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
            let challenge_prompt = build_challenge_prompt(
                &proposal_clone,
                &reviewer_name,
                &responsibility,
                round_num,
            );

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
            })
            .await?;

            let review = self
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
            repo_rt::create_message(
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
        if DivergenceCalculator::needs_human_intervention(divergence, self.config.divergence_threshold) {
            warn!("High divergence ({:.3}), suspending for human", divergence);
            self.emit_event(
                session_id,
                "agent_team:divergence_alert",
                json!({
                    "session_id": session_id,
                    "divergence_score": divergence,
                    "threshold": self.config.divergence_threshold
                }),
            );
            self.transition_state(session_id, TeamSessionState::SuspendedForHuman)
                .await?;
            repo_rt::complete_round(&pool, &round.id, Some(divergence)).await?;
            return Ok((divergence, true));
        }

        self.transition_state(session_id, TeamSessionState::ConvergenceCheck)
            .await?;
        repo_rt::complete_round(&pool, &round.id, Some(divergence)).await?;
        self.emit_event(
            session_id,
            "agent_team:round_completed",
            json!({"round": round_number, "phase": "challenging", "divergence_score": divergence}),
        );

        info!("Challenge phase round {} completed, divergence={:.3}", round_number, divergence);
        Ok((divergence, false))
    }

    /// Phase: DECIDING - 其他角色 Review 并形成最终方案
    async fn run_decide_phase(&self, session_id: &str, session: &AgentTeamSession) -> Result<()> {
        let db = self
            .app_handle
            .try_state::<Arc<DatabaseService>>()
            .context("DatabaseService not available")?;
        let pool = db.get_runtime_pool().context("Failed to get db pool")?;

        self.transition_state(session_id, TeamSessionState::Deciding)
            .await?;

        let round = repo_rt::create_round(&pool, session_id, 2, "deciding").await?;
        self.emit_event(
            session_id,
            "agent_team:round_started",
            json!({"round": 2, "phase": "deciding"}),
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
        let mut all_reviews = vec![format!("**{}的初始提案：**\n{}", session.members[0].name, &proposal)];

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
                2,
            );

            let context = build_role_context(RoleContextInput {
                app_handle: &self.app_handle,
                session_id,
                member: reviewer,
                session_goal: goal,
                round_task: &challenge_prompt,
                round_number: 2,
                llm_config: &llm_config,
                blackboard: &self.blackboard,
                directed_messages: vec![ChatMessage {
                    role: "user".to_string(),
                    content: challenge_prompt.clone(),
                    tool_calls: None,
                    tool_call_id: None,
                    reasoning_content: None,
                }],
            })
            .await?;

            let review = self
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

            repo_rt::create_message(
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
            warn!(
                "High divergence detected ({:.3}), suspending for human review",
                divergence
            );
            self.emit_event(
                session_id,
                "agent_team:divergence_alert",
                json!({
                    "session_id": session_id,
                    "divergence_score": divergence,
                    "threshold": self.config.divergence_threshold
                }),
            );
            self.transition_state(session_id, TeamSessionState::SuspendedForHuman)
                .await?;
            repo_rt::complete_round(&pool, &round.id, Some(divergence)).await?;
            return Ok(());
        }

        // 生成最终决策
        let blackboard_summary = self.blackboard.get_context_summary(session_id).await;
        let decide_prompt = build_decide_prompt(
            goal,
            &all_reviews.join("\n\n"),
            &blackboard_summary,
            2,
        );

        // 使用第1号角色作为决策整合者
        let decider = &session.members[0];
        let llm_config = self.get_llm_config_for_member(Some(decider)).await?;
        let context = build_role_context(RoleContextInput {
            app_handle: &self.app_handle,
            session_id,
            member: decider,
            session_goal: goal,
            round_task: &decide_prompt,
            round_number: 2,
            llm_config: &llm_config,
            blackboard: &self.blackboard,
            directed_messages: vec![ChatMessage {
                role: "user".to_string(),
                content: decide_prompt.clone(),
                tool_calls: None,
                tool_call_id: None,
                reasoning_content: None,
            }],
        })
        .await?;

        let decision = self
            .invoke_llm_streaming(
                session_id,
                Some(decider),
                "deciding",
                &llm_config,
                &context.system_prompt,
                &context.history_messages,
            )
            .await?;

        repo_rt::create_message(
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
            json!({"round": 2, "phase": "deciding", "divergence_score": divergence}),
        );

        info!("Deciding phase completed for session: {}", session_id);
        Ok(())
    }

    /// 生成标准文档产物（PRD / Architecture）
    async fn generate_artifacts(
        &self,
        session_id: &str,
        session: &AgentTeamSession,
    ) -> Result<()> {
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
            .map(|m| format!("[{}]: {}", m.member_name.as_deref().unwrap_or("系统"), shorten_text(&m.content, 300)))
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
        })
        .await?;

        let artifact_content = self
            .invoke_llm_streaming(
                session_id,
                Some(architect),
                "artifact_generation",
                &llm_config,
                &context.system_prompt,
                &context.history_messages,
            )
            .await?;

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
    fn emit_event(&self, session_id: &str, event: &str, payload: serde_json::Value) {
        let _ = self.app_handle.emit(event, &payload);
    }

    /// 获取 LLM 配置（支持角色模型覆盖，未配置回退到全局默认）
    async fn get_llm_config_for_member(&self, member: Option<&AgentTeamMember>) -> Result<LlmConfig> {
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
    ) -> Result<String> {
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
        let dynamic_tools = self.resolve_dynamic_tools_for_member(member).await;
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
                    StreamContent::ToolCallComplete { name, .. } => {
                        if let Some(mid) = member_id.as_deref() {
                            if let Ok(mut gov) = self.tool_governance.lock() {
                                gov.record_call(mid, &name);
                            }
                        }
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
                Ok(text)
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

        server.get_dynamic_tools(&selected_names).await
    }
} // end impl AgentTeamEngine

// ==================== 全局引擎管理 ====================

use std::collections::HashMap;
use tokio::sync::RwLock;
use std::sync::OnceLock;

static RUNNING_SESSIONS: OnceLock<RwLock<HashMap<String, tokio::task::JoinHandle<()>>>> =
    OnceLock::new();

fn get_running_sessions() -> &'static RwLock<HashMap<String, tokio::task::JoinHandle<()>>> {
    RUNNING_SESSIONS.get_or_init(|| RwLock::new(HashMap::new()))
}

/// 启动 Team 运行（异步，后台执行）
pub async fn start_agent_team_run_async(app_handle: AppHandle, session_id: String) -> Result<()> {
    let sessions = get_running_sessions();
    let is_running = {
        let lock = sessions.read().await;
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
            error!("Agent Team run failed for session {}: {:#}", session_id_clone, e);
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
                            error_message: Some(e.to_string()),
                        },
                    )
                    .await;
                }
            }
        }
    });

    let mut lock = sessions.write().await;
    lock.insert(session_id, handle);

    Ok(())
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
