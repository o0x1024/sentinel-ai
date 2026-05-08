use sentinel_llm::LlmClient;
use serde::{Deserialize, Serialize};
use std::sync::Arc;

use crate::services::ai::AiServiceManager;

const SCHEDULE_PARSER_PROMPT: &str = r#"You classify Weixin user messages for recurring schedule creation.

Return JSON only:
{
  "action": "create" | "none",
  "cron": string | null,
  "task": string | null,
  "reason": string | null
}

Rules:
1. Only use action="create" when the user clearly asks for a recurring schedule, recurring reminder, recurring subscription, or recurring push task.
2. Convert the schedule into a standard cron expression with 6 fields:
   second minute hour day_of_month month day_of_week
3. Never output "?" in cron. Use "*" instead.
4. Interpret the user's time in Asia/Shanghai timezone.
5. Map vague Chinese time phrases consistently:
   - "早上" => 08:00:00
   - "上午" => 09:00:00
   - "中午" => 12:00:00
   - "下午" => 15:00:00
   - "晚上" => 20:00:00
6. Supported recurring intents include daily, weekdays, weekly, monthly, and fixed intervals like every 2 hours.
7. task must keep the actual work to do, and remove the scheduling words.
8. If the message is a one-time reminder, casual question, or immediate execution request, return action="none".

Examples:
- "给我每天早上收集时事新闻然后发给我"
  => {"action":"create","cron":"0 0 8 * * *","task":"收集时事新闻并发给我","reason":"daily morning recurring task"}
- "工作日晚上 8 点给我总结安全新闻"
  => {"action":"create","cron":"0 0 20 * * 1-5","task":"总结安全新闻并发给我","reason":"weekday recurring task"}
- "现在帮我收集今天新闻"
  => {"action":"none","cron":null,"task":null,"reason":"immediate task, not recurring"}"#;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WeixinScheduleIntent {
    #[serde(default = "default_action")]
    pub action: String,
    #[serde(default)]
    pub cron: Option<String>,
    #[serde(default)]
    pub task: Option<String>,
    #[serde(default)]
    pub reason: Option<String>,
}

pub fn looks_like_schedule_text(text: &str) -> bool {
    let normalized = text.trim();
    if normalized.is_empty() {
        return false;
    }
    let has_direct_schedule_verb = ["定时", "提醒我", "推送", "订阅", "每隔", "cron"]
        .iter()
        .any(|keyword| normalized.contains(keyword));
    if has_direct_schedule_verb {
        return true;
    }

    let has_recurrence = [
        "每天",
        "每周",
        "每月",
        "工作日",
        "周一",
        "周二",
        "周三",
        "周四",
        "周五",
        "周六",
        "周日",
        "星期",
        "daily",
        "weekly",
        "every day",
        "every week",
        "every month",
    ]
    .iter()
    .any(|keyword| normalized.contains(keyword));
    if !has_recurrence {
        return false;
    }

    let has_delivery_intent = [
        "给我",
        "发给我",
        "发送给我",
        "推给我",
        "提醒",
        "收集",
        "总结",
        "汇总",
        "send me",
        "notify me",
    ]
    .iter()
    .any(|keyword| normalized.contains(keyword));
    let has_time_detail = [
        "早上",
        "上午",
        "中午",
        "下午",
        "晚上",
        "点",
        "every morning",
        "every evening",
    ]
    .iter()
    .any(|keyword| normalized.contains(keyword));

    has_delivery_intent || has_time_detail
}

pub async fn parse_schedule_intent(
    ai_manager: &Arc<AiServiceManager>,
    text: &str,
) -> Result<WeixinScheduleIntent, String> {
    let llm_config = ai_manager
        .resolve_generation_llm_config(None, None)
        .await
        .map_err(|e| e.to_string())?;
    let client = LlmClient::new(llm_config);
    let raw = client
        .completion(Some(SCHEDULE_PARSER_PROMPT), text)
        .await
        .map_err(|e| format!("Weixin schedule parser failed: {e}"))?;
    let json = extract_json_object(&raw)?;
    let intent: WeixinScheduleIntent = serde_json::from_str(json)
        .map_err(|e| format!("Weixin schedule JSON parse failed: {e}"))?;
    Ok(intent)
}

fn extract_json_object(raw: &str) -> Result<&str, String> {
    let start = raw
        .find('{')
        .ok_or_else(|| "schedule parser did not return JSON".to_string())?;
    let end = raw
        .rfind('}')
        .ok_or_else(|| "schedule parser returned incomplete JSON".to_string())?;
    if end <= start {
        return Err("schedule parser returned malformed JSON".to_string());
    }
    Ok(&raw[start..=end])
}

fn default_action() -> String {
    "none".to_string()
}
