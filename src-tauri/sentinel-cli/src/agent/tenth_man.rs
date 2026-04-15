use super::config::ContestLlmConfig;
use crate::arena::ChallengeInfo;
use anyhow::Result;
use sentinel_llm::{ChatMessage, LlmClient, LlmConfig};
use sentinel_tools::buildin_tools::tenth_man_tool::{
    set_tenth_man_executor, ReviewMode, TenthManExecutorFn, TenthManToolArgs, TenthManToolError,
    TenthManToolOutput,
};
use std::collections::HashMap;
use std::sync::{Arc, OnceLock};
use tokio::sync::RwLock;

const MAX_HISTORY_MESSAGES: usize = 16;
const MAX_MESSAGE_CHARS: usize = 1_200;
const MAX_SUMMARY_CHARS: usize = 4_000;

const QUICK_REVIEW_PROMPT: &str = r#"你是一个安全比赛解题代理的“第十人”审查者。

你的职责不是支持当前思路，而是主动反驳它。
请快速指出最可能导致当前思路失败的关键问题、被忽略的证据，以及更值得优先验证的替代方向。

要求：
- 直接、尖锐、简短
- 只讲与当前题目推进最相关的风险
- 如果当前方向没有明显问题，也要指出“最危险但最容易被忽略”的失败模式
- 使用简体中文输出"#;

const FULL_REVIEW_PROMPT: &str = r#"你是一个安全比赛解题代理的“第十人”审查者。

你必须假设当前分析可能是错的、不完整的、或者陷入了确认偏误。
请系统性挑战当前思路，重点寻找：
1. 被默认接受但未经验证的前提
2. 忽略的高价值攻击面
3. 工具调用上的重复、低效或误导性路径
4. 更稳妥、更高收益的替代验证路径

输出要求：
- 使用简体中文
- 直接批判，不要礼貌铺垫
- 结构固定为：
  1. 关键漏洞
  2. 隐含风险
  3. 替代路径
  4. 下一步最值得验证的单个动作"#;

#[derive(Debug, Clone)]
pub struct ContestReviewContext {
    pub llm: ContestLlmConfig,
    pub challenge: ChallengeInfo,
    pub entrypoints: Vec<String>,
    pub hint: Option<String>,
    pub context_summary: Option<String>,
    pub last_feedback: Option<String>,
    pub current_prompt: String,
    pub history: Vec<ChatMessage>,
    pub step_index: usize,
    pub max_steps: usize,
}

fn review_contexts() -> &'static Arc<RwLock<HashMap<String, ContestReviewContext>>> {
    static CONTEXTS: OnceLock<Arc<RwLock<HashMap<String, ContestReviewContext>>>> = OnceLock::new();
    CONTEXTS.get_or_init(|| Arc::new(RwLock::new(HashMap::new())))
}

pub fn init_tenth_man_executor() {
    static INIT: OnceLock<()> = OnceLock::new();
    INIT.get_or_init(|| {
        let executor: TenthManExecutorFn =
            Arc::new(|args| Box::pin(async move { execute_review(args).await }));
        set_tenth_man_executor(executor);
    });
}

pub async fn set_review_context(execution_id: String, context: ContestReviewContext) {
    review_contexts()
        .write()
        .await
        .insert(execution_id, context);
}

pub async fn clear_review_context(execution_id: &str) {
    review_contexts().write().await.remove(execution_id);
}

async fn execute_review(args: TenthManToolArgs) -> Result<TenthManToolOutput, TenthManToolError> {
    let context = review_contexts()
        .read()
        .await
        .get(&args.execution_id)
        .cloned()
        .ok_or_else(|| TenthManToolError::ConfigNotFound(args.execution_id.clone()))?;

    let system_prompt = match args.review_type.as_str() {
        "quick" => QUICK_REVIEW_PROMPT,
        _ => FULL_REVIEW_PROMPT,
    };
    let review_input = build_review_input(&context, &args);
    let client = build_review_client(&context.llm, &args.execution_id);
    let critique = client
        .completion(Some(system_prompt), &review_input)
        .await
        .map_err(|error| TenthManToolError::ReviewFailed(error.to_string()))?;

    Ok(TenthManToolOutput {
        success: true,
        risk_level: infer_risk_level(&critique),
        message: "tenth man review completed".to_string(),
        critique: Some(critique),
    })
}

fn build_review_client(config: &ContestLlmConfig, execution_id: &str) -> LlmClient {
    let mut llm_config = LlmConfig::new(&config.provider, &config.model)
        .with_timeout(config.timeout_secs)
        .with_rig_provider(&config.rig_provider)
        .with_conversation_id(format!("{}:tenth-man", execution_id))
        .with_max_turns(1);

    if let Some(api_key) = &config.api_key {
        llm_config = llm_config.with_api_key(api_key.clone());
    }
    if let Some(base_url) = &config.base_url {
        llm_config = llm_config.with_base_url(base_url.clone());
    }

    LlmClient::new(llm_config)
}

fn build_review_input(context: &ContestReviewContext, args: &TenthManToolArgs) -> String {
    let mut sections = vec![
        format!("execution_id: {}", args.execution_id),
        format!(
            "challenge: {} ({})",
            context.challenge.title, context.challenge.code
        ),
        format!("step: {}/{}", context.step_index, context.max_steps),
        format!("difficulty: {}", context.challenge.difficulty),
        "entrypoints:".to_string(),
    ];

    for entrypoint in &context.entrypoints {
        sections.push(format!("- {}", entrypoint));
    }

    sections.push("description:".to_string());
    sections.push(context.challenge.description.clone());

    if let Some(focus_area) = args.focus_area.as_deref() {
        sections.push("focus_area:".to_string());
        sections.push(focus_area.to_string());
    }

    if let Some(hint) = context.hint.as_deref() {
        sections.push("hint:".to_string());
        sections.push(hint.to_string());
    }

    if let Some(summary) = context.context_summary.as_deref() {
        sections.push("compressed_context_summary:".to_string());
        sections.push(truncate(summary, MAX_SUMMARY_CHARS));
    }

    if let Some(feedback) = context.last_feedback.as_deref() {
        sections.push("runner_feedback:".to_string());
        sections.push(feedback.to_string());
    }

    sections.push("current_turn_prompt:".to_string());
    sections.push(context.current_prompt.clone());

    sections.push("recent_history:".to_string());
    sections.push(format_history(&context.history, &args.review_mode));
    sections.join("\n")
}

fn format_history(history: &[ChatMessage], review_mode: &ReviewMode) -> String {
    match review_mode {
        ReviewMode::SpecificContent { content } => content.clone(),
        ReviewMode::RecentMessages { count } => render_messages(
            history
                .iter()
                .rev()
                .take(*count)
                .cloned()
                .collect::<Vec<_>>()
                .into_iter()
                .rev(),
        ),
        ReviewMode::FullHistory => {
            let take = history.len().min(MAX_HISTORY_MESSAGES);
            render_messages(
                history
                    .iter()
                    .rev()
                    .take(take)
                    .cloned()
                    .collect::<Vec<_>>()
                    .into_iter()
                    .rev(),
            )
        }
    }
}

fn render_messages<I>(messages: I) -> String
where
    I: IntoIterator<Item = ChatMessage>,
{
    let mut output = String::new();
    for message in messages {
        output.push_str(&format!(
            "[{}]\n{}\n\n",
            message.role,
            truncate(&message.content, MAX_MESSAGE_CHARS)
        ));
    }
    output
}

fn truncate(value: &str, max_chars: usize) -> String {
    let truncated = value.chars().take(max_chars).collect::<String>();
    if value.chars().count() > max_chars {
        format!("{}...[truncated]", truncated)
    } else {
        truncated
    }
}

fn infer_risk_level(critique: &str) -> String {
    let lower = critique.to_lowercase();
    if lower.contains("严重") || lower.contains("高风险") || lower.contains("critical") {
        "high".to_string()
    } else if lower.contains("中风险") || lower.contains("明显问题") || lower.contains("medium")
    {
        "medium".to_string()
    } else {
        "low".to_string()
    }
}
