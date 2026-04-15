use chrono::Utc;
use serde::{Deserialize, Serialize};
use sentinel_llm::ChatMessage;

use crate::agents::context_engineering::token_utils::estimate_message_tokens;
use crate::agents::context_engineering::tool_digest::{condense_text, ToolDigest};

const MAX_INTENT_TOKENS: usize = 18;
const MAX_ACTIVE_INTENTS: usize = 6;
const MAX_PINNED_ITEMS: usize = 10;
const MAX_COMPRESSION_ITEMS: usize = 12;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum SentinelIntentStatus {
    #[default]
    Active,
    Resolved,
    Suspended,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum SentinelIntentRelation {
    #[default]
    NewIntent,
    Continuation,
    ResumeOldIntent,
    Branch,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct SentinelIntentState {
    pub intent_id: String,
    pub goal: String,
    pub task_type: String,
    #[serde(default)]
    pub focus_objects: Vec<String>,
    #[serde(default)]
    pub constraints: Vec<String>,
    #[serde(default)]
    pub expected_output: Option<String>,
    #[serde(default)]
    pub continuation_of: Option<String>,
    pub confidence: f32,
    pub relation: SentinelIntentRelation,
    pub status: SentinelIntentStatus,
    #[serde(default)]
    pub clarification_needed: bool,
    pub created_at_ms: i64,
    pub updated_at_ms: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct SentinelPinnedContext {
    #[serde(default)]
    pub hard_constraints: Vec<String>,
    #[serde(default)]
    pub must_keep_facts: Vec<String>,
    #[serde(default)]
    pub must_keep_artifacts: Vec<String>,
    #[serde(default)]
    pub must_not_repeat_failures: Vec<String>,
    #[serde(default)]
    pub output_requirements: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct SentinelContextSlice {
    pub slice_id: String,
    pub intent_id: String,
    pub source_type: String,
    pub source_ref: String,
    pub relevance_score: f32,
    pub risk_level: String,
    pub keep_mode: String,
    pub content: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct SentinelCompressionState {
    pub version: i64,
    #[serde(default)]
    pub active_intent_ids: Vec<String>,
    #[serde(default)]
    pub resolved_intent_ids: Vec<String>,
    #[serde(default)]
    pub facts: Vec<String>,
    #[serde(default)]
    pub constraints: Vec<String>,
    #[serde(default)]
    pub decisions: Vec<String>,
    #[serde(default)]
    pub failed_attempts: Vec<String>,
    #[serde(default)]
    pub open_loops: Vec<String>,
    #[serde(default)]
    pub artifacts: Vec<String>,
    #[serde(default)]
    pub tool_findings: Vec<String>,
    #[serde(default)]
    pub slices: Vec<SentinelContextSlice>,
    #[serde(default)]
    pub summary_text: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct SentinelClarificationState {
    pub needed: bool,
    pub reason: String,
    pub confidence: f32,
    pub recommended_timeout_secs: u64,
    pub default_mode: String,
    pub recommended_timeout_policy: String,
    pub resolution_status: String,
    pub resolution_source: String,
    pub compression_aggressiveness: SentinelCompressionAggressiveness,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, Default, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum SentinelCompressionAggressiveness {
    High,
    #[default]
    Medium,
    Low,
    Disabled,
}

#[derive(Debug, Clone, Default)]
pub struct SentinelHistorySelection {
    pub kept_history: Vec<ChatMessage>,
    pub dropped_slices: Vec<SentinelContextSlice>,
    pub trim_trace: Vec<String>,
}

#[derive(Debug, Clone, Default)]
struct GroupedHistoryChunk {
    index: usize,
    score: f32,
    tokens: usize,
    messages: Vec<ChatMessage>,
    source_type: String,
    summary: String,
    risk_level: String,
}

pub fn analyze_intent(
    task: &str,
    existing_intents: &[SentinelIntentState],
) -> SentinelIntentState {
    let now_ms = Utc::now().timestamp_millis();
    let goal = condense_text(task.trim(), 220);
    let focus_objects = extract_focus_objects(task);
    let constraints = extract_constraints(task);
    let task_type = infer_task_type(task);
    let expected_output = infer_expected_output(task);
    let task_tokens = tokenize(task);

    let mut best_match: Option<&SentinelIntentState> = None;
    let mut best_overlap = 0.0f32;
    for intent in existing_intents.iter().rev().take(MAX_ACTIVE_INTENTS) {
        let overlap = token_overlap(&task_tokens, &tokenize(&intent.goal));
        if overlap > best_overlap {
            best_overlap = overlap;
            best_match = Some(intent);
        }
    }

    let continuation_hint = has_continuation_hint(task);
    let relation = if continuation_hint && best_match.is_some() {
        SentinelIntentRelation::Continuation
    } else if best_overlap >= 0.7 {
        SentinelIntentRelation::ResumeOldIntent
    } else if best_overlap >= 0.35 {
        SentinelIntentRelation::Continuation
    } else if has_branch_hint(task) && best_match.is_some() {
        SentinelIntentRelation::Branch
    } else {
        SentinelIntentRelation::NewIntent
    };

    let continuation_of = best_match.map(|intent| intent.intent_id.clone());
    let base_confidence: f32 = if continuation_hint {
        0.92
    } else if best_overlap >= 0.7 {
        0.9
    } else if best_overlap >= 0.35 {
        0.78
    } else if task_tokens.len() >= 6 {
        0.82
    } else {
        0.64
    };

    let confidence = (base_confidence
        + if !constraints.is_empty() { 0.05_f32 } else { 0.0_f32 }
        + if !focus_objects.is_empty() { 0.04_f32 } else { 0.0_f32 })
        .min(0.99_f32);
    let clarification_needed = confidence < 0.72;

    let intent_id = continuation_of.clone().unwrap_or_else(|| {
        let brief = goal
            .chars()
            .take(24)
            .collect::<String>()
            .replace(char::is_whitespace, "-")
            .to_lowercase();
        format!("intent-{}-{}", now_ms, brief)
    });
    let created_at_ms = best_match
        .filter(|_| relation != SentinelIntentRelation::NewIntent)
        .map(|intent| intent.created_at_ms)
        .unwrap_or(now_ms);

    SentinelIntentState {
        intent_id,
        goal,
        task_type,
        focus_objects,
        constraints,
        expected_output,
        continuation_of,
        confidence,
        relation,
        status: SentinelIntentStatus::Active,
        clarification_needed,
        created_at_ms,
        updated_at_ms: now_ms,
    }
}

pub fn update_intent_registry(
    intents: &mut Vec<SentinelIntentState>,
    current: &SentinelIntentState,
) {
    for intent in intents.iter_mut() {
        if intent.intent_id == current.intent_id {
            *intent = current.clone();
            return;
        }
        if intent.status == SentinelIntentStatus::Active
            && intent.intent_id != current.intent_id
            && current.relation == SentinelIntentRelation::NewIntent
        {
            intent.status = SentinelIntentStatus::Suspended;
        }
    }
    intents.push(current.clone());
    intents.sort_by_key(|item| item.updated_at_ms);
    if intents.len() > MAX_ACTIVE_INTENTS {
        let keep_from = intents.len() - MAX_ACTIVE_INTENTS;
        let truncated = intents.split_off(keep_from);
        *intents = truncated;
    }
}

pub fn update_pinned_context(
    pinned: &mut SentinelPinnedContext,
    task: &str,
    intent: &SentinelIntentState,
    compression: &SentinelCompressionState,
) {
    merge_unique_limited(
        &mut pinned.hard_constraints,
        extract_constraints(task),
        MAX_PINNED_ITEMS,
    );
    merge_unique_limited(
        &mut pinned.output_requirements,
        extract_output_requirements(task),
        6,
    );
    merge_unique_limited(
        &mut pinned.must_keep_facts,
        compression.facts.iter().take(3).cloned().collect(),
        MAX_PINNED_ITEMS,
    );
    merge_unique_limited(
        &mut pinned.must_not_repeat_failures,
        compression.failed_attempts.iter().take(3).cloned().collect(),
        MAX_PINNED_ITEMS,
    );
    merge_unique_limited(
        &mut pinned.must_keep_artifacts,
        compression.artifacts.iter().take(3).cloned().collect(),
        MAX_PINNED_ITEMS,
    );
    merge_unique_limited(
        &mut pinned.must_keep_facts,
        intent.focus_objects.clone(),
        MAX_PINNED_ITEMS,
    );
}

pub fn render_sentinel_context(
    intent: &SentinelIntentState,
    pinned: &SentinelPinnedContext,
    compression: &SentinelCompressionState,
    clarification: &SentinelClarificationState,
) -> String {
    let mut sections = Vec::new();
    sections.push(format!(
        "[Sentinel Intent]\n- intent_id: {}\n- relation: {:?}\n- confidence: {:.2}\n- task_type: {}\n- goal: {}",
        intent.intent_id,
        intent.relation,
        intent.confidence,
        intent.task_type,
        intent.goal
    ));

    if !intent.focus_objects.is_empty() {
        sections.push(format!(
            "[Sentinel Focus]\n- {}",
            intent.focus_objects.join("\n- ")
        ));
    }

    if clarification.needed {
        sections.push(format!(
            "[Sentinel Clarification]\n- confidence_low: true\n- reason: {}\n- recommended_timeout_secs: {}\n- recommended_timeout_policy: {}\n- default_mode: {}\n- current_resolution_status: {}\n- current_resolution_source: {}\n- compression_aggressiveness: {:?}\n- When ambiguity blocks progress or could rewrite prior intent structure, call ask_user_question before taking the aggressive path. For low-risk continuation questions, prefer timeout_policy=`use_default` with a safe default answer. Otherwise prefer timeout_policy=`return_timeout`.",
            clarification.reason,
            clarification.recommended_timeout_secs,
            clarification.recommended_timeout_policy,
            clarification.default_mode,
            clarification.resolution_status,
            clarification.resolution_source,
            clarification.compression_aggressiveness,
        ));
    } else {
        sections.push(format!(
            "[Sentinel Clarification State]\n- resolution_status: {}\n- resolution_source: {}\n- compression_aggressiveness: {:?}",
            clarification.resolution_status,
            clarification.resolution_source,
            clarification.compression_aggressiveness,
        ));
    }

    let pinned_lines = render_pinned_context(pinned);
    if !pinned_lines.is_empty() {
        sections.push(pinned_lines);
    }

    let compressed_lines = render_compression_state(compression);
    if !compressed_lines.is_empty() {
        sections.push(compressed_lines);
    }

    sections.join("\n\n")
}

pub fn build_sentinel_clarification_state(intent: &SentinelIntentState) -> SentinelClarificationState {
    SentinelClarificationState {
        needed: intent.clarification_needed,
        reason: if intent.clarification_needed {
            "intent confidence below sentinel threshold".to_string()
        } else {
            String::new()
        },
        confidence: intent.confidence,
        recommended_timeout_secs: 20,
        default_mode: if intent.relation == SentinelIntentRelation::Continuation {
            "continue_current_intent".to_string()
        } else {
            "return_timeout".to_string()
        },
        recommended_timeout_policy: if intent.relation == SentinelIntentRelation::Continuation {
            "use_default".to_string()
        } else {
            "return_timeout".to_string()
        },
        resolution_status: "pending".to_string(),
        resolution_source: "none".to_string(),
        compression_aggressiveness: if intent.clarification_needed {
            SentinelCompressionAggressiveness::Low
        } else if intent.confidence >= 0.9 {
            SentinelCompressionAggressiveness::High
        } else {
            SentinelCompressionAggressiveness::Medium
        },
    }
}

pub fn reconcile_sentinel_clarification(
    mut clarification: SentinelClarificationState,
    tool_digests: &[ToolDigest],
) -> SentinelClarificationState {
    let ask_digest = tool_digests
        .iter()
        .rev()
        .find(|digest| digest.tool_name == "ask_user_question");

    let Some(digest) = ask_digest else {
        return clarification;
    };
    let Some(metadata) = digest.metadata.as_ref().and_then(|value| value.as_object()) else {
        return clarification;
    };

    let resolution_status = metadata
        .get("status")
        .and_then(|value| value.as_str())
        .unwrap_or("resolved");
    let resolution_source = metadata
        .get("source")
        .and_then(|value| value.as_str())
        .unwrap_or("user");

    clarification.resolution_status = resolution_status.to_string();
    clarification.resolution_source = resolution_source.to_string();
    match resolution_status {
        "resolved" => {
            clarification.needed = false;
            clarification.reason.clear();
            clarification.compression_aggressiveness = if resolution_source == "user" {
                SentinelCompressionAggressiveness::High
            } else {
                SentinelCompressionAggressiveness::Medium
            };
        }
        "timeout_with_default" => {
            clarification.needed = false;
            clarification.reason =
                "ask_user_question timed out; continuing with system defaults".to_string();
            clarification.compression_aggressiveness = SentinelCompressionAggressiveness::Low;
        }
        "timeout_without_default" => {
            clarification.needed = true;
            clarification.reason =
                "ask_user_question timed out without a safe answer; keep context preservation conservative"
                    .to_string();
            clarification.compression_aggressiveness = SentinelCompressionAggressiveness::Disabled;
        }
        _ => {}
    }

    clarification
}

pub fn apply_sentinel_history_selection(
    history: &[ChatMessage],
    intent: &SentinelIntentState,
    clarification: &SentinelClarificationState,
    available_for_history: usize,
    compression: &mut SentinelCompressionState,
) -> SentinelHistorySelection {
    if clarification.compression_aggressiveness == SentinelCompressionAggressiveness::Disabled {
        return SentinelHistorySelection {
            kept_history: history.to_vec(),
            dropped_slices: Vec::new(),
            trim_trace: vec!["sentinel_compression_disabled".to_string()],
        };
    }

    let mut groups = group_history(history);
    let total_tokens = groups.iter().map(|group| group.tokens).sum::<usize>();
    let target_history_budget =
        adjusted_history_budget(available_for_history, total_tokens, clarification);
    if total_tokens <= target_history_budget {
        return SentinelHistorySelection {
            kept_history: history.to_vec(),
            dropped_slices: Vec::new(),
            trim_trace: Vec::new(),
        };
    }

    let intent_tokens = tokenize(&format!(
        "{}\n{}\n{}",
        intent.goal,
        intent.constraints.join(" "),
        intent.focus_objects.join(" ")
    ));
    for group in &mut groups {
        group.score = score_group(group, &intent_tokens, intent.confidence);
    }

    let mut sorted = groups
        .iter()
        .map(|group| (group.index, group.score))
        .collect::<Vec<_>>();
    sorted.sort_by(|a, b| a.1.partial_cmp(&b.1).unwrap_or(std::cmp::Ordering::Equal));

    let mut keep = vec![true; groups.len()];
    let mut kept_tokens = total_tokens;
    let mut dropped_slices = Vec::new();
    let mut trim_trace = Vec::new();

    for (index, _) in sorted {
        if kept_tokens <= target_history_budget {
            break;
        }
        if groups.len().saturating_sub(keep.iter().filter(|flag| **flag).count()) >= groups.len() {
            break;
        }
        if index + 1 == groups.len() {
            continue;
        }
        if groups[index].score >= score_guard_threshold(clarification, intent.confidence) {
            continue;
        }
        keep[index] = false;
        kept_tokens = kept_tokens.saturating_sub(groups[index].tokens);
        dropped_slices.push(build_context_slice(&groups[index], &intent.intent_id));
        trim_trace.push(format!("sentinel_drop_group_{}", index));
    }

    let kept_history = groups
        .iter()
        .enumerate()
        .filter(|(index, _)| keep[*index])
        .flat_map(|(_, group)| group.messages.clone())
        .collect::<Vec<_>>();

    if !dropped_slices.is_empty() {
        merge_compression_state(compression, intent, &dropped_slices);
    }

    SentinelHistorySelection {
        kept_history,
        dropped_slices,
        trim_trace,
    }
}

fn adjusted_history_budget(
    available_for_history: usize,
    total_tokens: usize,
    clarification: &SentinelClarificationState,
) -> usize {
    let relaxed = match clarification.compression_aggressiveness {
        SentinelCompressionAggressiveness::High => available_for_history,
        SentinelCompressionAggressiveness::Medium => {
            available_for_history.saturating_add((available_for_history / 10).max(128))
        }
        SentinelCompressionAggressiveness::Low => {
            available_for_history.saturating_add((available_for_history / 4).max(256))
        }
        SentinelCompressionAggressiveness::Disabled => total_tokens,
    };
    relaxed.min(total_tokens)
}

fn score_guard_threshold(
    clarification: &SentinelClarificationState,
    confidence: f32,
) -> f32 {
    match clarification.compression_aggressiveness {
        SentinelCompressionAggressiveness::High => {
            if confidence < 0.72 { 2.4 } else { 2.1 }
        }
        SentinelCompressionAggressiveness::Medium => {
            if confidence < 0.72 { 2.2 } else { 1.95 }
        }
        SentinelCompressionAggressiveness::Low => {
            if confidence < 0.72 { 1.8 } else { 1.65 }
        }
        SentinelCompressionAggressiveness::Disabled => f32::MAX,
    }
}

fn render_pinned_context(pinned: &SentinelPinnedContext) -> String {
    let mut lines = Vec::new();
    if !pinned.hard_constraints.is_empty() {
        lines.push(format!(
            "Hard Constraints:\n- {}",
            pinned.hard_constraints.join("\n- ")
        ));
    }
    if !pinned.must_keep_facts.is_empty() {
        lines.push(format!(
            "Must-Keep Facts:\n- {}",
            pinned.must_keep_facts.join("\n- ")
        ));
    }
    if !pinned.must_keep_artifacts.is_empty() {
        lines.push(format!(
            "Must-Keep Artifacts:\n- {}",
            pinned.must_keep_artifacts.join("\n- ")
        ));
    }
    if !pinned.must_not_repeat_failures.is_empty() {
        lines.push(format!(
            "Must-Not-Repeat Failures:\n- {}",
            pinned.must_not_repeat_failures.join("\n- ")
        ));
    }
    if !pinned.output_requirements.is_empty() {
        lines.push(format!(
            "Output Requirements:\n- {}",
            pinned.output_requirements.join("\n- ")
        ));
    }
    if lines.is_empty() {
        String::new()
    } else {
        format!("[Sentinel Pinned Context]\n{}", lines.join("\n"))
    }
}

fn render_compression_state(compression: &SentinelCompressionState) -> String {
    if compression.version <= 0
        && compression.facts.is_empty()
        && compression.constraints.is_empty()
        && compression.failed_attempts.is_empty()
        && compression.open_loops.is_empty()
        && compression.slices.is_empty()
    {
        return String::new();
    }

    let mut lines = vec![format!("- version: {}", compression.version)];
    if !compression.facts.is_empty() {
        lines.push(format!(
            "- facts:\n  - {}",
            compression.facts.join("\n  - ")
        ));
    }
    if !compression.constraints.is_empty() {
        lines.push(format!(
            "- constraints:\n  - {}",
            compression.constraints.join("\n  - ")
        ));
    }
    if !compression.failed_attempts.is_empty() {
        lines.push(format!(
            "- failed_attempts:\n  - {}",
            compression.failed_attempts.join("\n  - ")
        ));
    }
    if !compression.open_loops.is_empty() {
        lines.push(format!(
            "- open_loops:\n  - {}",
            compression.open_loops.join("\n  - ")
        ));
    }
    if !compression.slices.is_empty() {
        let rendered = compression
            .slices
            .iter()
            .take(5)
            .map(|slice| format!("- [{}:{}] {}", slice.source_type, slice.keep_mode, slice.content))
            .collect::<Vec<_>>()
            .join("\n");
        lines.push(format!("- slices:\n{}", rendered));
    }
    if !compression.summary_text.trim().is_empty() {
        lines.push(format!("- summary: {}", compression.summary_text.trim()));
    }

    format!("[Sentinel Compressed Context]\n{}", lines.join("\n"))
}

fn merge_compression_state(
    compression: &mut SentinelCompressionState,
    intent: &SentinelIntentState,
    slices: &[SentinelContextSlice],
) {
    compression.version += 1;
    merge_unique_limited(
        &mut compression.active_intent_ids,
        vec![intent.intent_id.clone()],
        MAX_ACTIVE_INTENTS,
    );
    merge_unique_limited(
        &mut compression.constraints,
        intent.constraints.clone(),
        MAX_COMPRESSION_ITEMS,
    );
    merge_unique_limited(
        &mut compression.open_loops,
        vec![format!("Continue intent {}: {}", intent.intent_id, intent.goal)],
        MAX_COMPRESSION_ITEMS,
    );
    for slice in slices {
        if slice.risk_level == "high" && slice.content.contains("failed") {
            merge_unique_limited(
                &mut compression.failed_attempts,
                vec![slice.content.clone()],
                MAX_COMPRESSION_ITEMS,
            );
        }
        if slice.source_type == "tool" {
            merge_unique_limited(
                &mut compression.tool_findings,
                vec![slice.content.clone()],
                MAX_COMPRESSION_ITEMS,
            );
        }
        if contains_artifact_like_content(&slice.content) {
            merge_unique_limited(
                &mut compression.artifacts,
                vec![slice.content.clone()],
                MAX_COMPRESSION_ITEMS,
            );
        }
        merge_unique_limited(
            &mut compression.facts,
            vec![slice.content.clone()],
            MAX_COMPRESSION_ITEMS,
        );
        compression.slices.push(slice.clone());
    }
    if compression.slices.len() > MAX_COMPRESSION_ITEMS {
        let keep_from = compression.slices.len() - MAX_COMPRESSION_ITEMS;
        compression.slices = compression.slices.split_off(keep_from);
    }
    compression.summary_text = condense_text(
        &compression
            .slices
            .iter()
            .rev()
            .take(4)
            .map(|slice| slice.content.clone())
            .collect::<Vec<_>>()
            .join(" | "),
        420,
    );
}

fn group_history(history: &[ChatMessage]) -> Vec<GroupedHistoryChunk> {
    let mut groups = Vec::new();
    let mut index = 0usize;
    while index < history.len() {
        let current = &history[index];
        if current.role == "assistant" && current.tool_calls.is_some() {
            let mut messages = vec![current.clone()];
            let source_type = "tool".to_string();
            let mut next = index + 1;
            while next < history.len() && history[next].role == "tool" {
                messages.push(history[next].clone());
                next += 1;
            }
            let summary = summarize_chunk(&messages);
            let risk_level = infer_risk_level(&summary);
            let tokens = messages.iter().map(estimate_message_tokens).sum();
            groups.push(GroupedHistoryChunk {
                index: groups.len(),
                score: 0.0,
                tokens,
                messages,
                source_type,
                summary,
                risk_level,
            });
            index = next;
            continue;
        }

        let messages = vec![current.clone()];
        let summary = summarize_chunk(&messages);
        let risk_level = infer_risk_level(&summary);
        let tokens = estimate_message_tokens(current);
        let source_type = current.role.clone();
        groups.push(GroupedHistoryChunk {
            index: groups.len(),
            score: 0.0,
            tokens,
            messages,
            source_type,
            summary,
            risk_level,
        });
        index += 1;
    }
    groups
}

fn score_group(
    group: &GroupedHistoryChunk,
    intent_tokens: &[String],
    confidence: f32,
) -> f32 {
    let summary_tokens = tokenize(&group.summary);
    let overlap = token_overlap(intent_tokens, &summary_tokens);
    let recency_score = 1.0 + (group.index as f32 * 0.08);
    let risk_bonus = match group.risk_level.as_str() {
        "high" => 1.2,
        "medium" => 0.6,
        _ => 0.0,
    };
    let confidence_bias = if confidence < 0.72 { 0.5 } else { 0.0 };
    overlap * 4.0 + recency_score + risk_bonus + confidence_bias
}

fn summarize_chunk(messages: &[ChatMessage]) -> String {
    let mut parts = Vec::new();
    for message in messages {
        let role = message.role.trim();
        let content = message.content.trim();
        if content.is_empty() {
            continue;
        }
        parts.push(format!("{}: {}", role, condense_text(content, 180)));
    }
    condense_text(&parts.join(" | "), 240)
}

fn build_context_slice(
    chunk: &GroupedHistoryChunk,
    intent_id: &str,
) -> SentinelContextSlice {
    SentinelContextSlice {
        slice_id: format!("slice-{}-{}", intent_id, chunk.index),
        intent_id: intent_id.to_string(),
        source_type: chunk.source_type.clone(),
        source_ref: format!("history_group_{}", chunk.index),
        relevance_score: chunk.score,
        risk_level: chunk.risk_level.clone(),
        keep_mode: if chunk.risk_level == "high" {
            "archive".to_string()
        } else {
            "compress".to_string()
        },
        content: chunk.summary.clone(),
    }
}

fn infer_task_type(task: &str) -> String {
    let lower = task.to_lowercase();
    if contains_any(&lower, &["fix", "repair", "bug", "修复", "排查", "debug"]) {
        "fix".to_string()
    } else if contains_any(&lower, &["review", "audit", "评审", "审查", "reviewer"]) {
        "review".to_string()
    } else if contains_any(&lower, &["implement", "build", "写", "实现", "开发"]) {
        "implementation".to_string()
    } else if contains_any(&lower, &["explain", "why", "说明", "解释"]) {
        "explanation".to_string()
    } else if contains_any(&lower, &["compare", "区别", "对比"]) {
        "comparison".to_string()
    } else if contains_any(&lower, &["continue", "继续", "接着", "下一步"]) {
        "continuation".to_string()
    } else {
        "general".to_string()
    }
}

fn infer_expected_output(task: &str) -> Option<String> {
    let lower = task.to_lowercase();
    if contains_any(&lower, &["plan", "方案", "计划"]) {
        Some("plan".to_string())
    } else if contains_any(&lower, &["code", "patch", "实现", "修改"]) {
        Some("code_change".to_string())
    } else if contains_any(&lower, &["summary", "总结", "概述"]) {
        Some("summary".to_string())
    } else {
        None
    }
}

fn extract_focus_objects(task: &str) -> Vec<String> {
    let mut objects = tokenize(task)
        .into_iter()
        .filter(|token| token.len() >= 3)
        .take(MAX_INTENT_TOKENS)
        .collect::<Vec<_>>();
    let lower = task.to_lowercase();
    for marker in [".rs", ".ts", ".vue", ".json", ".md", "/", "http", "sql"] {
        if lower.contains(marker) {
            objects.push(marker.to_string());
        }
    }
    dedupe_limit(&mut objects, MAX_INTENT_TOKENS);
    objects
}

fn extract_constraints(task: &str) -> Vec<String> {
    let mut items = Vec::new();
    for sentence in split_sentences(task) {
        let lower = sentence.to_lowercase();
        if contains_any(
            &lower,
            &["must", "must not", "do not", "不要", "必须", "不能", "only", "只保留"],
        ) {
            items.push(condense_text(sentence.trim(), 160));
        }
    }
    dedupe_limit(&mut items, 8);
    items
}

fn extract_output_requirements(task: &str) -> Vec<String> {
    let mut items = Vec::new();
    for sentence in split_sentences(task) {
        let lower = sentence.to_lowercase();
        if contains_any(
            &lower,
            &["output", "format", "格式", "只保留", "不要", "plain text", "markdown"],
        ) {
            items.push(condense_text(sentence.trim(), 160));
        }
    }
    dedupe_limit(&mut items, 6);
    items
}

fn tokenize(text: &str) -> Vec<String> {
    let mut tokens = text
        .to_lowercase()
        .split(|ch: char| !ch.is_alphanumeric() && !is_cjk(ch))
        .map(str::trim)
        .filter(|token| token.len() >= 2)
        .map(str::to_string)
        .collect::<Vec<_>>();
    dedupe_limit(&mut tokens, 64);
    tokens
}

fn token_overlap(left: &[String], right: &[String]) -> f32 {
    if left.is_empty() || right.is_empty() {
        return 0.0;
    }
    let mut hits = 0usize;
    for token in left {
        if right.iter().any(|item| item == token) {
            hits += 1;
        }
    }
    hits as f32 / left.len().max(right.len()) as f32 * 2.0
}

fn has_continuation_hint(task: &str) -> bool {
    let lower = task.to_lowercase();
    contains_any(
        &lower,
        &["continue", "继续", "接着", "based on", "延续", "follow up", "next step"],
    )
}

fn has_branch_hint(task: &str) -> bool {
    let lower = task.to_lowercase();
    contains_any(&lower, &["另外", "另一个", "instead", "branch", "改成", "换个"])
}

fn infer_risk_level(text: &str) -> String {
    let lower = text.to_lowercase();
    if contains_any(
        &lower,
        &["fail", "error", "timeout", "rejected", "forbidden", "失败", "错误", "超时"],
    ) {
        "high".to_string()
    } else if contains_any(
        &lower,
        &["must", "constraint", "requirement", "artifact", "路径", "约束"],
    ) {
        "medium".to_string()
    } else {
        "low".to_string()
    }
}

fn contains_artifact_like_content(text: &str) -> bool {
    let lower = text.to_lowercase();
    contains_any(
        &lower,
        &[".rs", ".ts", ".vue", "/", "artifact", "stdout", "stderr", "response", "request"],
    )
}

fn split_sentences(text: &str) -> Vec<&str> {
    text.split(['\n', '.', '。', '!', '！', '?', '？'])
        .map(str::trim)
        .filter(|part| !part.is_empty())
        .collect()
}

fn contains_any(text: &str, needles: &[&str]) -> bool {
    needles.iter().any(|needle| text.contains(needle))
}

fn merge_unique_limited(target: &mut Vec<String>, mut incoming: Vec<String>, limit: usize) {
    for item in incoming.drain(..) {
        let normalized = item.trim();
        if normalized.is_empty() || target.iter().any(|existing| existing == normalized) {
            continue;
        }
        target.push(normalized.to_string());
    }
    dedupe_limit(target, limit);
}

fn dedupe_limit(items: &mut Vec<String>, limit: usize) {
    let mut out = Vec::new();
    for item in items.drain(..) {
        let normalized = item.trim();
        if normalized.is_empty() || out.iter().any(|existing: &String| existing == normalized) {
            continue;
        }
        out.push(normalized.to_string());
        if out.len() >= limit {
            break;
        }
    }
    *items = out;
}

fn is_cjk(ch: char) -> bool {
    matches!(
        ch as u32,
        0x4E00..=0x9FFF
            | 0x3400..=0x4DBF
            | 0x20000..=0x2A6DF
            | 0x2A700..=0x2B73F
            | 0x2B740..=0x2B81F
            | 0x2B820..=0x2CEAF
            | 0xF900..=0xFAFF
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sentinel_intent_detects_continuation_hints() {
        let existing = vec![SentinelIntentState {
            intent_id: "intent-1".to_string(),
            goal: "fix login timeout in api gateway".to_string(),
            confidence: 0.9,
            status: SentinelIntentStatus::Active,
            ..SentinelIntentState::default()
        }];
        let intent = analyze_intent("继续修复 login timeout 的问题", &existing);
        assert_eq!(intent.relation, SentinelIntentRelation::Continuation);
        assert!(intent.continuation_of.is_some());
        assert!(intent.confidence > 0.8);
    }

    #[test]
    fn sentinel_history_selection_prefers_relevant_groups() {
        let intent = SentinelIntentState {
            intent_id: "intent-1".to_string(),
            goal: "fix login timeout in api gateway".to_string(),
            confidence: 0.92,
            ..SentinelIntentState::default()
        };
        let clarification = SentinelClarificationState {
            compression_aggressiveness: SentinelCompressionAggressiveness::High,
            ..build_sentinel_clarification_state(&intent)
        };
        let history = vec![
            ChatMessage::user("random unrelated note"),
            ChatMessage::assistant("nothing important here"),
            ChatMessage::user("login timeout still fails on api gateway"),
        ];
        let mut compression = SentinelCompressionState::default();
        let selected = apply_sentinel_history_selection(
            &history,
            &intent,
            &clarification,
            24,
            &mut compression,
        );
        assert!(!selected.kept_history.is_empty());
        assert!(selected
            .kept_history
            .iter()
            .any(|message| message.content.contains("login timeout")));
    }

    #[test]
    fn sentinel_timeout_without_default_disables_aggressive_compression() {
        let base = SentinelClarificationState {
            needed: true,
            confidence: 0.66,
            compression_aggressiveness: SentinelCompressionAggressiveness::Low,
            ..Default::default()
        };
        let clarified = reconcile_sentinel_clarification(
            base,
            &[ToolDigest {
                tool_name: "ask_user_question".to_string(),
                status: "timeout".to_string(),
                summary: "AskUserQuestion timed out without answers".to_string(),
                artifact_id: None,
                artifact_kind: None,
                preview_snippets: Vec::new(),
                metadata: Some(serde_json::json!({
                    "status": "timeout_without_default",
                    "source": "timeout"
                })),
                created_at_ms: 0,
            }],
        );

        assert!(clarified.needed);
        assert_eq!(
            clarified.compression_aggressiveness,
            SentinelCompressionAggressiveness::Disabled
        );
    }
}
