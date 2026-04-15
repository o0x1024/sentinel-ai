use super::signal::{extract_candidate_flags, parse_agent_signal, AgentSignalStatus};
use sentinel_llm::ChatMessage;
use serde::Serialize;

const MAX_HISTORY_MESSAGES: usize = 10;
const RECENT_HISTORY_MESSAGES: usize = 6;
const MAX_SUMMARY_CHARS: usize = 4_000;
const MAX_MESSAGE_CHARS: usize = 4_000;
const MAX_TOTAL_HISTORY_CHARS: usize = 18_000;

#[derive(Debug, Clone, Serialize)]
pub struct CompactionReport {
    pub summarized_messages: usize,
    pub remaining_messages: usize,
    pub summary_chars: usize,
}

pub fn compact_history_if_needed(
    history: &mut Vec<ChatMessage>,
    rolling_summary: &mut Option<String>,
) -> Option<CompactionReport> {
    truncate_history_messages(history);
    enforce_total_history_chars(history);

    if history.len() <= MAX_HISTORY_MESSAGES {
        return None;
    }

    let split_index = history.len().saturating_sub(RECENT_HISTORY_MESSAGES);
    if split_index == 0 {
        return None;
    }

    let older_messages = history[..split_index].to_vec();
    let mut next_summary = rolling_summary.clone().unwrap_or_default();
    let chunk_summary = summarize_messages(&older_messages);

    if !chunk_summary.is_empty() {
        if !next_summary.is_empty() {
            next_summary.push_str("\n");
        }
        next_summary.push_str(&chunk_summary);
        if next_summary.len() > MAX_SUMMARY_CHARS {
            let start = next_summary.len().saturating_sub(MAX_SUMMARY_CHARS);
            next_summary = next_summary[start..].to_string();
        }
        *rolling_summary = Some(next_summary.clone());
    }

    *history = history.split_off(split_index);

    Some(CompactionReport {
        summarized_messages: older_messages.len(),
        remaining_messages: history.len(),
        summary_chars: rolling_summary
            .as_deref()
            .map(|value| value.len())
            .unwrap_or(0),
    })
}

fn summarize_messages(messages: &[ChatMessage]) -> String {
    let mut lines = Vec::new();

    for message in messages {
        match message.role.as_str() {
            "user" => {
                if let Some(line) = summarize_user_message(message.content.as_str()) {
                    lines.push(line);
                }
            }
            "assistant" => {
                if let Some(line) = summarize_assistant_message(message.content.as_str()) {
                    lines.push(line);
                }
            }
            _ => {}
        }
    }

    dedupe_preserve_order(lines).join("\n")
}

fn summarize_user_message(content: &str) -> Option<String> {
    let trimmed = content.trim();
    if trimmed.is_empty() {
        return None;
    }

    if trimmed.starts_with("step:") {
        return trimmed
            .lines()
            .find(|line| line.starts_with("step:"))
            .map(|line| format!("User {}", line.trim()));
    }

    if trimmed.starts_with("Hint fetched successfully.") {
        return Some("Runner feedback: hint was fetched and injected.".to_string());
    }

    if trimmed.starts_with("Submitted candidate flag but it was incorrect") {
        return Some(trim_line(
            "Runner feedback: incorrect flag submission.",
            trimmed,
        ));
    }

    if trimmed.starts_with("Flag ") && trimmed.contains("already submitted earlier") {
        return Some(trim_line(
            "Runner feedback: duplicate flag candidate.",
            trimmed,
        ));
    }

    if trimmed.starts_with("agent response was not valid JSON signal") {
        return Some("Runner feedback: agent emitted invalid JSON signal once.".to_string());
    }

    None
}

fn summarize_assistant_message(content: &str) -> Option<String> {
    let trimmed = content.trim();
    if trimmed.is_empty() {
        return None;
    }

    if let Ok(signal) = parse_agent_signal(trimmed) {
        let status = match signal.status {
            AgentSignalStatus::Continue => "continue",
            AgentSignalStatus::NeedHint => "need_hint",
            AgentSignalStatus::CandidateFlag => "candidate_flag",
            AgentSignalStatus::Done => "done",
            AgentSignalStatus::GiveUp => "give_up",
        };
        let mut line = format!("Agent signal: {}", status);
        if let Some(reason) = signal
            .reason
            .as_deref()
            .filter(|value| !value.trim().is_empty())
        {
            line.push_str(" | ");
            line.push_str(&truncate(reason, 180));
        }
        if let Some(flag) = signal
            .flag
            .as_deref()
            .filter(|value| !value.trim().is_empty())
        {
            line.push_str(" | ");
            line.push_str(flag);
        }
        return Some(line);
    }

    let flags = extract_candidate_flags(trimmed);
    if !flags.is_empty() {
        return Some(format!(
            "Agent mentioned candidate flags: {}",
            flags.join(", ")
        ));
    }

    None
}

fn dedupe_preserve_order(lines: Vec<String>) -> Vec<String> {
    let mut seen = std::collections::BTreeSet::new();
    let mut result = Vec::new();
    for line in lines {
        if seen.insert(line.clone()) {
            result.push(line);
        }
    }
    result
}

fn trim_line(prefix: &str, value: &str) -> String {
    format!("{} {}", prefix, truncate(value, 180))
}

fn truncate(value: &str, max_chars: usize) -> String {
    if value.chars().count() <= max_chars {
        return value.to_string();
    }

    let truncated = value.chars().take(max_chars).collect::<String>();
    format!("{}...", truncated)
}

fn truncate_history_messages(history: &mut [ChatMessage]) {
    for message in history {
        if message.content.chars().count() > MAX_MESSAGE_CHARS {
            message.content = truncate(&message.content, MAX_MESSAGE_CHARS);
        }
    }
}

fn enforce_total_history_chars(history: &mut Vec<ChatMessage>) {
    let mut total_chars = history
        .iter()
        .map(|message| message.content.chars().count())
        .sum::<usize>();

    if total_chars <= MAX_TOTAL_HISTORY_CHARS {
        return;
    }

    let keep_from = history.len().saturating_sub(RECENT_HISTORY_MESSAGES);
    for message in &mut history[keep_from..] {
        if total_chars <= MAX_TOTAL_HISTORY_CHARS {
            break;
        }

        let current_len = message.content.chars().count();
        if current_len <= MAX_MESSAGE_CHARS / 2 {
            continue;
        }

        let target_len = (current_len / 2).max(MAX_MESSAGE_CHARS / 2);
        let trimmed = truncate(&message.content, target_len);
        total_chars = total_chars.saturating_sub(current_len) + trimmed.chars().count();
        message.content = trimmed;
    }
}
