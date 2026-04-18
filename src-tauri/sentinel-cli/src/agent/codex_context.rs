use crate::runtime::ChallengeRunState;
use sentinel_llm::ChatMessage;

use super::tooling::ContestToolDigest;

const RUN_STATE_MAX_CHARS: usize = 1_800;
const TOOL_DIGESTS_MAX_CHARS: usize = 1_400;
const WINDOW_MAX_CHARS: usize = 10_000;
const MAX_TOOL_DIGESTS: usize = 4;

pub fn build_codex_like_history(
    history: &[ChatMessage],
    run_state: &ChallengeRunState,
    context_summary: Option<&str>,
    last_feedback: Option<&str>,
    tool_digests: &[ContestToolDigest],
) -> Vec<ChatMessage> {
    let mut context_messages = Vec::new();

    if let Some(run_state_block) = render_run_state_block(run_state) {
        context_messages.push(ChatMessage::user(run_state_block));
    }

    if let Some(summary_block) = render_summary_block(context_summary, last_feedback, history) {
        context_messages.push(ChatMessage::user(summary_block));
    }

    if let Some(tool_digest_block) = render_tool_digest_block(tool_digests) {
        context_messages.push(ChatMessage::user(tool_digest_block));
    }

    let mut window_messages = trim_window_messages(history, WINDOW_MAX_CHARS);
    context_messages.append(&mut window_messages);
    context_messages
}

fn render_run_state_block(run_state: &ChallengeRunState) -> Option<String> {
    let mut lines = Vec::new();

    if let Some(status) = run_state.last_status.as_deref() {
        lines.push(format!("status: {}", status));
    }
    if let Some(attempt_id) = run_state.current_attempt_id.as_deref() {
        lines.push(format!("attempt_id: {}", attempt_id));
    }
    if let Some(step) = run_state.current_step {
        lines.push(format!("current_step: {}", step));
    }
    if run_state.hint_used {
        lines.push("hint_used: true".to_string());
    }
    if !run_state.entrypoints.is_empty() {
        lines.push(format!("entrypoints: {}", run_state.entrypoints.join(", ")));
    }
    if !run_state.discovered_flags.is_empty() {
        lines.push(format!(
            "discovered_flags: {}",
            run_state
                .discovered_flags
                .iter()
                .cloned()
                .collect::<Vec<_>>()
                .join(", ")
        ));
    }
    if !run_state.accepted_flags.is_empty() {
        lines.push(format!(
            "accepted_flags: {}",
            run_state
                .accepted_flags
                .iter()
                .cloned()
                .collect::<Vec<_>>()
                .join(", ")
        ));
    }
    if run_state.consecutive_failures > 0 {
        lines.push(format!(
            "consecutive_failures: {}",
            run_state.consecutive_failures
        ));
    }
    if let Some(next_eligible_at) = run_state.next_eligible_at.as_deref() {
        lines.push(format!("next_eligible_at: {}", next_eligible_at));
    }
    if run_state.tool_calls_count > 0 {
        lines.push(format!("tool_calls_count: {}", run_state.tool_calls_count));
    }
    if run_state.watchdog_interventions > 0 {
        lines.push(format!(
            "watchdog_interventions: {}",
            run_state.watchdog_interventions
        ));
    }

    if lines.is_empty() {
        return None;
    }

    Some(truncate_block(
        format!("[RunState]\n{}", lines.join("\n")),
        RUN_STATE_MAX_CHARS,
    ))
}

fn render_summary_block(
    context_summary: Option<&str>,
    last_feedback: Option<&str>,
    history: &[ChatMessage],
) -> Option<String> {
    let mut lines = Vec::new();

    if let Some(summary) = context_summary
        .map(str::trim)
        .filter(|value| !value.is_empty())
    {
        lines.push("[CompressedSummary]".to_string());
        lines.push(summary.to_string());
    }

    if let Some(feedback) = last_feedback
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .filter(|feedback| !history_already_contains_feedback(history, feedback))
    {
        if !lines.is_empty() {
            lines.push(String::new());
        }
        lines.push("[RunnerFeedback]".to_string());
        lines.push(feedback.to_string());
    }

    if lines.is_empty() {
        return None;
    }

    Some(truncate_block(lines.join("\n"), RUN_STATE_MAX_CHARS))
}

fn render_tool_digest_block(tool_digests: &[ContestToolDigest]) -> Option<String> {
    if tool_digests.is_empty() {
        return None;
    }

    let mut lines = vec!["[Recent Tool Digests]".to_string()];
    for digest in tool_digests.iter().take(MAX_TOOL_DIGESTS) {
        lines.push(format!(
            "- [{}] {}: {}",
            digest.status, digest.tool_name, digest.summary
        ));
    }

    Some(truncate_block(lines.join("\n"), TOOL_DIGESTS_MAX_CHARS))
}

fn trim_window_messages(history: &[ChatMessage], max_chars: usize) -> Vec<ChatMessage> {
    let mut trimmed = history.to_vec();
    let mut total_chars = total_history_chars(&trimmed);

    while total_chars > max_chars && !trimmed.is_empty() {
        total_chars = total_chars.saturating_sub(trimmed[0].content.chars().count());
        trimmed.remove(0);
    }

    trimmed
}

fn total_history_chars(history: &[ChatMessage]) -> usize {
    history
        .iter()
        .map(|message| message.content.chars().count())
        .sum()
}

fn history_already_contains_feedback(history: &[ChatMessage], feedback: &str) -> bool {
    history
        .last()
        .is_some_and(|message| message.role == "user" && message.content.trim() == feedback.trim())
}

fn truncate_block(value: String, max_chars: usize) -> String {
    if value.chars().count() <= max_chars {
        return value;
    }

    let truncated = value.chars().take(max_chars).collect::<String>();
    format!("{}...", truncated)
}
