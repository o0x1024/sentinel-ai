use sentinel_llm::LlmClient;
use serde::{Deserialize, Serialize};
use std::sync::Arc;

use crate::services::ai::AiServiceManager;

const INTENT_CLASSIFIER_PROMPT: &str = r#"You classify user messages into one of three categories and extract structured data when applicable.

Categories:
1. "create_mission" — The user wants a recurring, long-running task. Examples: monitoring websites, daily news digests, trend tracking, periodic reports, automated checks. Any request that implies repeated execution over time.
2. "create_schedule" — The user wants a simple recurring reminder or one-shot scheduled task without tracking/monitoring semantics. Examples: "提醒我每天喝水", "工作日 9 点叫我起床".
3. "none" — Everything else: one-time questions, casual chat, immediate requests, commands.

Return JSON only:
{
  "action": "create_mission" | "create_schedule" | "none",
  "title": string | null,
  "objective": string | null,
  "cron": string | null,
  "task": string | null,
  "context_strategy": "stateless" | "incremental" | "cumulative",
  "reason": string,
  "missing_info": string | null
}

Field rules:
- For "create_mission": fill title, objective, cron, context_strategy. task=null.
- For "create_schedule": fill cron, task (the work to do). title=null, objective=null.
- For "none": all fields null except reason.
- cron: 6-field format (second minute hour day_of_month month day_of_week). Never use "?", use "*" instead.
- Interpret times in Asia/Shanghai timezone.
- Time phrase mapping: "早上"=08:00, "上午"=09:00, "中午"=12:00, "下午"=15:00, "晚上"=20:00.
- context_strategy: "stateless" for independent runs, "incremental" for change detection, "cumulative" for trend/history analysis.
- missing_info: if critical info is missing (URL for monitoring, etc.), put a follow-up question here.

Examples:
- "帮我监控 example.com 有没有更新，每小时看一次"
  => {"action":"create_mission","title":"网站更新监控: example.com","objective":"每小时检查 example.com 的内容变化，有更新时通知","cron":"0 0 * * * *","task":null,"context_strategy":"incremental","reason":"website monitoring","missing_info":null}
- "每天早上给我总结一下 AI 新闻"
  => {"action":"create_mission","title":"每日 AI 新闻摘要","objective":"搜索并总结当日 AI 领域重要新闻","cron":"0 0 8 * * *","task":null,"context_strategy":"stateless","reason":"daily recurring information task","missing_info":null}
- "每天晚上 8 点提醒我锻炼"
  => {"action":"create_schedule","title":null,"objective":null,"cron":"0 0 20 * * *","task":"提醒锻炼","context_strategy":"stateless","reason":"simple recurring reminder","missing_info":null}
- "帮我查一下今天的天气"
  => {"action":"none","title":null,"objective":null,"cron":null,"task":null,"context_strategy":"stateless","reason":"one-time immediate request","missing_info":null}
- "长期帮我关注这个项目的安全漏洞"
  => {"action":"create_mission","title":"安全漏洞监控","objective":"持续关注目标项目的安全漏洞，发现新漏洞时及时通知","cron":"0 0 9 * * *","task":null,"context_strategy":"incremental","reason":"ongoing security monitoring","missing_info":"请提供需要监控的项目名称或地址"}"#;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UserIntent {
    #[serde(default = "default_action")]
    pub action: String,
    #[serde(default)]
    pub title: Option<String>,
    #[serde(default)]
    pub objective: Option<String>,
    #[serde(default)]
    pub cron: Option<String>,
    #[serde(default)]
    pub task: Option<String>,
    #[serde(default = "default_context_strategy")]
    pub context_strategy: String,
    #[serde(default)]
    pub reason: Option<String>,
    #[serde(default)]
    pub missing_info: Option<String>,
}

fn default_action() -> String {
    "none".to_string()
}
fn default_context_strategy() -> String {
    "stateless".to_string()
}

/// Classify user message intent via LLM — no keyword heuristics.
pub async fn classify_user_intent(
    ai_manager: &Arc<AiServiceManager>,
    text: &str,
) -> Result<UserIntent, String> {
    let llm_config = ai_manager
        .resolve_generation_llm_config(None, None)
        .await
        .map_err(|e| e.to_string())?;
    let client = LlmClient::new(llm_config);
    let raw = client
        .completion(Some(INTENT_CLASSIFIER_PROMPT), text)
        .await
        .map_err(|e| format!("Intent classifier failed: {e}"))?;
    let json = extract_json_object(&raw)?;
    let intent: UserIntent =
        serde_json::from_str(json).map_err(|e| format!("Intent JSON parse failed: {e}"))?;
    Ok(intent)
}

fn extract_json_object(raw: &str) -> Result<&str, String> {
    let start = raw
        .find('{')
        .ok_or_else(|| "intent classifier did not return JSON".to_string())?;
    let end = raw
        .rfind('}')
        .ok_or_else(|| "intent classifier returned incomplete JSON".to_string())?;
    if end <= start {
        return Err("intent classifier returned malformed JSON".to_string());
    }
    Ok(&raw[start..=end])
}
