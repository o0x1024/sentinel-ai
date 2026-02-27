//! 角色上下文桥接模块
//!
//! 为每个角色构建"专属 System Prompt + 共享白板摘要"的隔离上下文，
//! 防止多角色讨论导致的 Token 爆炸。

use anyhow::Result;
use sentinel_db::Database;
use sentinel_llm::{ChatMessage, LlmConfig};
use tauri::{AppHandle, Manager};
use tracing::info;

use super::blackboard::BlackboardManager;
use super::models::AgentTeamMember;

/// 角色专属上下文构建输入
pub struct RoleContextInput<'a> {
    pub app_handle: &'a AppHandle,
    pub session_id: &'a str,
    pub member: &'a AgentTeamMember,
    pub session_goal: &'a str,
    /// 本轮任务描述
    pub round_task: &'a str,
    /// 当前轮次号
    pub round_number: i32,
    pub llm_config: &'a LlmConfig,
    /// 白板管理器引用
    pub blackboard: &'a BlackboardManager,
    /// 本角色可看到的定向消息（由调度器注入）
    pub directed_messages: Vec<ChatMessage>,
    /// 共享执行记忆（由引擎注入，帮助后续角色避免重复工作）
    pub shared_execution_memory: Option<&'a str>,
}

/// 角色专属上下文构建结果
pub struct RoleContextResult {
    /// 完整 System Prompt（角色专属 + 白板摘要）
    pub system_prompt: String,
    /// 可用于本次 LLM 调用的历史消息
    pub history_messages: Vec<ChatMessage>,
}

/// 为指定角色构建专属上下文
///
/// # 核心设计
/// - 角色只能看到：专属 System Prompt + 共享白板快照 + 与自己相关的定向路由消息
/// - 不播放全量对话历史，防止 Token 爆炸
pub async fn build_role_context(input: RoleContextInput<'_>) -> Result<RoleContextResult> {
    let member = input.member;

    // 1. 基础角色 System Prompt
    let base_system_prompt = member
        .system_prompt
        .as_deref()
        .unwrap_or("你是一位专业的团队成员，请基于讨论内容提供专业意见。");

    // 2. 角色元信息注入
    let role_meta = format!(
        "\n\n[角色信息]\n- 角色名称: {}\n- 职责: {}\n- 决策风格: {}\n- 风险偏好: {}\n- 权重: {:.1}",
        member.name,
        member.responsibility.as_deref().unwrap_or("未指定"),
        member.decision_style.as_deref().unwrap_or("balanced"),
        member.risk_preference.as_deref().unwrap_or("medium"),
        member.weight,
    );

    // 3. Team 协作规则注入
    let collaboration_rules = "\n\n[Team 工作流协作规则]\n\
        当前节点任务由用户消息提供；system 仅定义稳定协作约束。\n\
        \n\
        协作原则:\n\
        1. 你正在执行工作流中的一个节点，只对当前节点目标负责，不扩展到未分配范围\n\
        2. 输出必须可交接给下游节点，优先给出可执行结论与必要证据\n\
        3. 当需要事实验证或执行操作时，优先使用可用工具（若工具可用）再给结论\n\
        4. 明确标注假设、风险和边界条件，避免模糊表述\n\
        5. 若发现上游输入不足，请明确指出缺口与最小补充信息\n\
        6. 回复末尾使用固定结构：\n\
           - **本节点结论**: [本节点可交付结果]\n\
           - **关键依据/风险**: [证据与风险]\n\
           - **下游交接**: [下一节点可直接使用的信息]\n"
        .to_string();

    // 4. 工作目录注入（与 AI 助手一致，优先 agent.working_directory）
    let working_dir_section = if let Some(db) = input
        .app_handle
        .try_state::<std::sync::Arc<sentinel_db::DatabaseService>>()
    {
        let configured_agent = db
            .inner()
            .get_config("agent", "working_directory")
            .await
            .ok()
            .flatten()
            .filter(|dir| !dir.trim().is_empty());
        let configured_legacy_ai = db
            .inner()
            .get_config("ai", "working_directory")
            .await
            .ok()
            .flatten()
            .filter(|dir| !dir.trim().is_empty());
        let working_dir = configured_agent.or(configured_legacy_ai);
        if let Some(dir) = working_dir {
            format!(
                "\n\n[Execution Environment]\n\
                - Working Directory: {}\n\
                \n\
                [Working Directory Note: When performing file operations, executing scripts, or any file system related tasks, use this directory as your base path unless explicitly specified otherwise by the user.]",
                dir
            )
        } else {
            String::new()
        }
    } else {
        String::new()
    };

    // 4. 共享白板快照注入（核心反 Token 爆炸机制）
    let blackboard_summary = input.blackboard.get_context_summary(input.session_id).await;
    let blackboard_section = format!("\n\n{}", blackboard_summary);

    // 5. 共享执行记忆（跨角色通用摘要卡片）
    let shared_memory_section = input
        .shared_execution_memory
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .map(|s| format!("\n\n[Shared Execution Memory]\n{}", s))
        .unwrap_or_default();

    // 6. 组装完整 System Prompt
    let system_prompt = format!(
        "{}{}{}{}{}{}",
        base_system_prompt,
        role_meta,
        collaboration_rules,
        working_dir_section,
        blackboard_section,
        shared_memory_section
    );

    // 7. 构建历史消息（仅包含定向路由消息，不含全量历史）
    let mut history_messages = input.directed_messages;

    // 确保消息不超过合理限制（防止 Token 超标）
    let max_history = 20;
    if history_messages.len() > max_history {
        let start = history_messages.len() - max_history;
        history_messages = history_messages[start..].to_vec();
    }

    info!(
        "Role context built for member '{}' in session '{}' round {}",
        member.name, input.session_id, input.round_number
    );

    Ok(RoleContextResult {
        system_prompt,
        history_messages,
    })
}

/// 生成角色初始 Propose 的提示词
pub fn build_propose_prompt(
    session_goal: &str,
    round_number: i32,
    member_name: &str,
    member_responsibility: &str,
) -> String {
    format!(
        "请作为「{}」（职责：{}），针对以下目标提出你的初始方案：\n\n**目标**：{}\n\n\
        请提供：\n\
        1. 核心方案概述（300字以内）\n\
        2. 关键决策点列表（3-5个）\n\
        3. 潜在风险（从你的职责视角）\n\
        4. 建议的下一步行动\n\
        \n\
        这是第{}轮讨论，请直接给出你的专业判断。",
        member_name, member_responsibility, session_goal, round_number
    )
}

/// 生成角色 Challenge 的提示词（用于 Review 他人提案）
pub fn build_challenge_prompt(
    proposal_to_review: &str,
    reviewer_name: &str,
    reviewer_responsibility: &str,
    round_number: i32,
) -> String {
    format!(
        "请作为「{}」（职责：{}），对以下提案进行专业评审和挑战：\n\n\
        **待评审提案**：\n{}\n\n\
        请从你的职责视角：\n\
        1. 指出你认同的要点（最多3个）\n\
        2. 指出你的异议（量化风险，必须给出具体数据或理由）\n\
        3. 提出具体改进建议\n\
        4. 给出该方案的综合评分（1-10分）及评分理由\n\
        \n\
        第{}轮评审。请直接、犀利地指出问题，这是建设性讨论。",
        reviewer_name, reviewer_responsibility, proposal_to_review, round_number
    )
}

/// 生成最终决策合并的提示词
pub fn build_decide_prompt(
    session_goal: &str,
    all_proposals: &str,
    blackboard_summary: &str,
    round_number: i32,
) -> String {
    format!(
        "请综合所有角色的发言，形成最终共识决策。\n\n\
        **会话目标**：{}\n\n\
        **各角色观点**：\n{}\n\n\
        **当前白板状态**：\n{}\n\n\
        请生成：\n\
        1. **最终方案**（综合各方意见的最优解）\n\
        2. **达成共识的要点**（更新到白板）\n\
        3. **遗留分歧**（需人工裁决的问题）\n\
        4. **行动计划**（具体可执行的下一步，带负责方）\n\
        5. **文档产物需求**（应生成哪些标准文档）\n\
        \n\
        这是第{}轮的最终决策环节。",
        session_goal, all_proposals, blackboard_summary, round_number
    )
}

/// 计算分歧度（Divergence Score）
///
/// 基于各角色评分的标准差。分数越高说明分歧越大。
/// 返回 0.0 ~ 1.0 的分歧度。
pub fn calculate_divergence_score(scores: &[f64]) -> f64 {
    if scores.is_empty() {
        return 0.0;
    }
    if scores.len() == 1 {
        return 0.0;
    }

    let mean = scores.iter().sum::<f64>() / scores.len() as f64;
    let variance = scores.iter().map(|s| (s - mean).powi(2)).sum::<f64>() / scores.len() as f64;
    let std_dev = variance.sqrt();

    // 归一化到 0~1，假设最大分歧是满分（10分）的标准差约为 5
    (std_dev / 5.0).min(1.0)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_divergence_score_no_divergence() {
        let scores = vec![8.0, 8.0, 8.0, 8.0];
        let score = calculate_divergence_score(&scores);
        assert!(score < 0.01, "同质评分应该几乎没有分歧: {}", score);
    }

    #[test]
    fn test_divergence_score_high_divergence() {
        let scores = vec![1.0, 10.0, 1.0, 10.0];
        let score = calculate_divergence_score(&scores);
        assert!(score > 0.5, "高度分歧的评分应该有较高分歧度: {}", score);
    }

    #[test]
    fn test_divergence_score_empty() {
        let score = calculate_divergence_score(&[]);
        assert_eq!(score, 0.0);
    }
}
