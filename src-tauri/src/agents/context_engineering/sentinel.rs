use chrono::Utc;
use sentinel_llm::ChatMessage;
use serde::{Deserialize, Serialize};

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
pub enum SentinelIntentTransition {
    #[default]
    Created,
    Continued,
    Resumed,
    Branched,
    Suspended,
    Resolved,
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
    pub parent_intent_id: Option<String>,
    #[serde(default)]
    pub focus_objects: Vec<String>,
    #[serde(default)]
    pub constraints: Vec<String>,
    #[serde(default)]
    pub expected_output: Option<String>,
    #[serde(default)]
    pub continuation_of: Option<String>,
    #[serde(default)]
    pub resumed_from_intent_id: Option<String>,
    pub confidence: f32,
    pub relation: SentinelIntentRelation,
    pub status: SentinelIntentStatus,
    #[serde(default)]
    pub last_transition: SentinelIntentTransition,
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

pub fn analyze_intent(task: &str, existing_intents: &[SentinelIntentState]) -> SentinelIntentState {
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
    let branch_hint = has_branch_hint(task);
    let relation = if branch_hint && best_match.is_some() {
        SentinelIntentRelation::Branch
    } else if continuation_hint && best_match.is_some() {
        SentinelIntentRelation::Continuation
    } else if best_overlap >= 0.7 {
        SentinelIntentRelation::ResumeOldIntent
    } else if best_overlap >= 0.35 {
        SentinelIntentRelation::Continuation
    } else {
        SentinelIntentRelation::NewIntent
    };

    let matched_intent_id = best_match.map(|intent| intent.intent_id.clone());
    let parent_intent_id = if relation == SentinelIntentRelation::Branch {
        matched_intent_id.clone()
    } else {
        None
    };
    let continuation_of = match relation {
        SentinelIntentRelation::Continuation => matched_intent_id.clone(),
        SentinelIntentRelation::Branch => matched_intent_id.clone(),
        _ => None,
    };
    let resumed_from_intent_id = if relation == SentinelIntentRelation::ResumeOldIntent {
        matched_intent_id.clone()
    } else {
        None
    };
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
        + if !constraints.is_empty() {
            0.05_f32
        } else {
            0.0_f32
        }
        + if !focus_objects.is_empty() {
            0.04_f32
        } else {
            0.0_f32
        })
    .min(0.99_f32);
    let clarification_needed = confidence < 0.72;

    let intent_id = match relation {
        SentinelIntentRelation::Continuation | SentinelIntentRelation::ResumeOldIntent => {
            matched_intent_id
                .clone()
                .unwrap_or_else(|| build_intent_id(now_ms, &goal))
        }
        SentinelIntentRelation::Branch | SentinelIntentRelation::NewIntent => {
            build_intent_id(now_ms, &goal)
        }
    };
    let created_at_ms = best_match
        .filter(|_| {
            matches!(
                relation,
                SentinelIntentRelation::Continuation | SentinelIntentRelation::ResumeOldIntent
            )
        })
        .map(|intent| intent.created_at_ms)
        .unwrap_or(now_ms);

    SentinelIntentState {
        intent_id,
        goal,
        task_type,
        parent_intent_id,
        focus_objects,
        constraints,
        expected_output,
        continuation_of,
        resumed_from_intent_id,
        confidence,
        relation,
        status: SentinelIntentStatus::Active,
        last_transition: transition_for_relation(relation),
        clarification_needed,
        created_at_ms,
        updated_at_ms: now_ms,
    }
}

pub fn update_intent_registry(
    intents: &mut Vec<SentinelIntentState>,
    current: &SentinelIntentState,
) {
    let mut replaced = false;
    for intent in intents.iter_mut() {
        if intent.intent_id == current.intent_id {
            *intent = current.clone();
            replaced = true;
            continue;
        }
        if intent.status == SentinelIntentStatus::Active && intent.intent_id != current.intent_id {
            intent.status = SentinelIntentStatus::Suspended;
            intent.last_transition = SentinelIntentTransition::Suspended;
            intent.updated_at_ms = current.updated_at_ms;
        }
    }
    if !replaced {
        intents.push(current.clone());
    }
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
        compression
            .failed_attempts
            .iter()
            .take(3)
            .cloned()
            .collect(),
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

pub fn focus_compression_state_for_intent(
    compression: &SentinelCompressionState,
    intent: &SentinelIntentState,
) -> SentinelCompressionState {
    if compression.slices.is_empty() {
        return compression.clone();
    }

    let intent_ids = related_intent_ids(intent);
    let intent_tokens = tokenize(&format!(
        "{}\n{}\n{}",
        intent.goal,
        intent.constraints.join(" "),
        intent.focus_objects.join(" ")
    ));

    let exact_slices = compression
        .slices
        .iter()
        .filter(|slice| {
            intent_ids
                .iter()
                .any(|intent_id| intent_id == &slice.intent_id)
        })
        .cloned()
        .collect::<Vec<_>>();
    let mut matching_slices = if !exact_slices.is_empty() {
        exact_slices
    } else {
        compression
            .slices
            .iter()
            .filter(|slice| token_overlap(&intent_tokens, &tokenize(&slice.content)) >= 0.18)
            .cloned()
            .collect::<Vec<_>>()
    };

    if matching_slices.is_empty() {
        matching_slices = compression
            .slices
            .iter()
            .rev()
            .take(2)
            .cloned()
            .collect::<Vec<_>>();
        matching_slices.reverse();
    }

    matching_slices.sort_by(|left, right| {
        right
            .relevance_score
            .partial_cmp(&left.relevance_score)
            .unwrap_or(std::cmp::Ordering::Equal)
    });
    matching_slices.truncate(6);

    let slice_contents = matching_slices
        .iter()
        .map(|slice| slice.content.clone())
        .collect::<Vec<_>>();
    let facts = slice_contents.iter().take(6).cloned().collect::<Vec<_>>();
    let failed_attempts = matching_slices
        .iter()
        .filter(|slice| slice.risk_level == "high")
        .map(|slice| slice.content.clone())
        .take(4)
        .collect::<Vec<_>>();
    let artifacts = matching_slices
        .iter()
        .filter(|slice| contains_artifact_like_content(&slice.content))
        .map(|slice| slice.content.clone())
        .take(4)
        .collect::<Vec<_>>();
    let constraints = compression
        .constraints
        .iter()
        .filter(|item| token_overlap(&intent_tokens, &tokenize(item)) >= 0.12)
        .take(6)
        .cloned()
        .collect::<Vec<_>>();
    let open_loops = compression
        .open_loops
        .iter()
        .filter(|item| {
            item.contains(&intent.intent_id)
                || intent
                    .continuation_of
                    .as_ref()
                    .is_some_and(|previous| item.contains(previous))
                || token_overlap(&intent_tokens, &tokenize(item)) >= 0.12
        })
        .take(6)
        .cloned()
        .collect::<Vec<_>>();

    SentinelCompressionState {
        version: compression.version,
        active_intent_ids: compression
            .active_intent_ids
            .iter()
            .filter(|item| intent_ids.iter().any(|intent_id| intent_id == *item))
            .cloned()
            .collect(),
        resolved_intent_ids: compression
            .resolved_intent_ids
            .iter()
            .filter(|item| intent_ids.iter().any(|intent_id| intent_id == *item))
            .cloned()
            .collect(),
        facts,
        constraints,
        decisions: compression
            .decisions
            .iter()
            .filter(|item| token_overlap(&intent_tokens, &tokenize(item)) >= 0.12)
            .take(4)
            .cloned()
            .collect(),
        failed_attempts,
        open_loops,
        artifacts,
        tool_findings: compression
            .tool_findings
            .iter()
            .filter(|item| token_overlap(&intent_tokens, &tokenize(item)) >= 0.12)
            .take(4)
            .cloned()
            .collect(),
        slices: matching_slices.clone(),
        summary_text: condense_text(&slice_contents.join(" | "), 420),
    }
}

pub fn restore_pinned_context_for_intent(
    pinned: &mut SentinelPinnedContext,
    compression: &SentinelCompressionState,
    intent: &SentinelIntentState,
) {
    let focused = focus_compression_state_for_intent(compression, intent);
    merge_unique_limited(
        &mut pinned.must_keep_facts,
        focused.facts.iter().take(4).cloned().collect(),
        MAX_PINNED_ITEMS,
    );
    merge_unique_limited(
        &mut pinned.must_keep_artifacts,
        focused.artifacts.iter().take(4).cloned().collect(),
        MAX_PINNED_ITEMS,
    );
    merge_unique_limited(
        &mut pinned.must_not_repeat_failures,
        focused.failed_attempts.iter().take(4).cloned().collect(),
        MAX_PINNED_ITEMS,
    );
    merge_unique_limited(
        &mut pinned.hard_constraints,
        focused.constraints.iter().take(4).cloned().collect(),
        MAX_PINNED_ITEMS,
    );
}

pub fn render_sentinel_context(
    intent: &SentinelIntentState,
    pinned: &SentinelPinnedContext,
    compression: &SentinelCompressionState,
    clarification: &SentinelClarificationState,
    intent_registry: &[SentinelIntentState],
) -> String {
    let mut sections = Vec::new();
    sections.push(format!(
        "[Sentinel Intent]\n- intent_id: {}\n- relation: {:?}\n- transition: {:?}\n- confidence: {:.2}\n- task_type: {}\n- goal: {}",
        intent.intent_id,
        intent.relation,
        intent.last_transition,
        intent.confidence,
        intent.task_type,
        intent.goal
    ));

    if let Some(parent_intent_id) = intent.parent_intent_id.as_ref() {
        sections.push(format!(
            "[Sentinel Parent]\n- parent_intent_id: {}",
            parent_intent_id
        ));
    }
    if let Some(resumed_from) = intent.resumed_from_intent_id.as_ref() {
        sections.push(format!(
            "[Sentinel Resume]\n- resumed_from_intent_id: {}",
            resumed_from
        ));
    }

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

    let registry_lines = render_intent_registry(intent_registry, intent);
    if !registry_lines.is_empty() {
        sections.push(registry_lines);
    }

    sections.join("\n\n")
}

pub fn build_sentinel_clarification_state(
    intent: &SentinelIntentState,
) -> SentinelClarificationState {
    SentinelClarificationState {
        needed: intent.clarification_needed,
        reason: if intent.clarification_needed {
            "intent confidence below sentinel threshold".to_string()
        } else {
            String::new()
        },
        confidence: intent.confidence,
        recommended_timeout_secs: 120,
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
        if groups
            .len()
            .saturating_sub(keep.iter().filter(|flag| **flag).count())
            >= groups.len()
        {
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

fn related_intent_ids(intent: &SentinelIntentState) -> Vec<String> {
    let mut ids = vec![intent.intent_id.clone()];
    if let Some(previous) = intent.continuation_of.as_ref() {
        if !ids.iter().any(|item| item == previous) {
            ids.push(previous.clone());
        }
    }
    if let Some(parent) = intent.parent_intent_id.as_ref() {
        if !ids.iter().any(|item| item == parent) {
            ids.push(parent.clone());
        }
    }
    if let Some(resumed_from) = intent.resumed_from_intent_id.as_ref() {
        if !ids.iter().any(|item| item == resumed_from) {
            ids.push(resumed_from.clone());
        }
    }
    ids
}

fn build_intent_id(now_ms: i64, goal: &str) -> String {
    let brief = goal
        .chars()
        .take(24)
        .collect::<String>()
        .replace(char::is_whitespace, "-")
        .to_lowercase();
    format!("intent-{}-{}", now_ms, brief)
}

fn transition_for_relation(relation: SentinelIntentRelation) -> SentinelIntentTransition {
    match relation {
        SentinelIntentRelation::NewIntent => SentinelIntentTransition::Created,
        SentinelIntentRelation::Continuation => SentinelIntentTransition::Continued,
        SentinelIntentRelation::ResumeOldIntent => SentinelIntentTransition::Resumed,
        SentinelIntentRelation::Branch => SentinelIntentTransition::Branched,
    }
}

fn score_guard_threshold(clarification: &SentinelClarificationState, confidence: f32) -> f32 {
    match clarification.compression_aggressiveness {
        SentinelCompressionAggressiveness::High => {
            if confidence < 0.72 {
                2.4
            } else {
                2.1
            }
        }
        SentinelCompressionAggressiveness::Medium => {
            if confidence < 0.72 {
                2.2
            } else {
                1.95
            }
        }
        SentinelCompressionAggressiveness::Low => {
            if confidence < 0.72 {
                1.8
            } else {
                1.65
            }
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
            .map(|slice| {
                format!(
                    "- [{}:{}] {}",
                    slice.source_type, slice.keep_mode, slice.content
                )
            })
            .collect::<Vec<_>>()
            .join("\n");
        lines.push(format!("- slices:\n{}", rendered));
    }
    if !compression.summary_text.trim().is_empty() {
        lines.push(format!("- summary: {}", compression.summary_text.trim()));
    }

    format!("[Sentinel Compressed Context]\n{}", lines.join("\n"))
}

fn render_intent_registry(
    intents: &[SentinelIntentState],
    current_intent: &SentinelIntentState,
) -> String {
    if intents.is_empty() {
        return String::new();
    }
    let mut lines = Vec::new();
    for intent in intents.iter().rev().take(4) {
        let marker = if intent.intent_id == current_intent.intent_id {
            "*"
        } else {
            "-"
        };
        lines.push(format!(
            "{} {} [{:?}/{:?}] {}",
            marker,
            intent.intent_id,
            intent.status,
            intent.last_transition,
            condense_text(&intent.goal, 72)
        ));
    }
    format!("[Sentinel Intent Registry]\n{}", lines.join("\n"))
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
        vec![format!(
            "Continue intent {}: {}",
            intent.intent_id, intent.goal
        )],
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

fn score_group(group: &GroupedHistoryChunk, intent_tokens: &[String], confidence: f32) -> f32 {
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

fn build_context_slice(chunk: &GroupedHistoryChunk, intent_id: &str) -> SentinelContextSlice {
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
            &[
                "must",
                "must not",
                "do not",
                "不要",
                "必须",
                "不能",
                "only",
                "只保留",
            ],
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
            &[
                "output",
                "format",
                "格式",
                "只保留",
                "不要",
                "plain text",
                "markdown",
            ],
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
        &[
            "continue",
            "继续",
            "接着",
            "based on",
            "延续",
            "follow up",
            "next step",
        ],
    )
}

fn has_branch_hint(task: &str) -> bool {
    let lower = task.to_lowercase();
    contains_any(
        &lower,
        &["另外", "另一个", "instead", "branch", "改成", "换个"],
    )
}

fn infer_risk_level(text: &str) -> String {
    let lower = text.to_lowercase();
    if contains_any(
        &lower,
        &[
            "fail",
            "error",
            "timeout",
            "rejected",
            "forbidden",
            "失败",
            "错误",
            "超时",
        ],
    ) {
        "high".to_string()
    } else if contains_any(
        &lower,
        &[
            "must",
            "constraint",
            "requirement",
            "artifact",
            "路径",
            "约束",
        ],
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
        &[
            ".rs", ".ts", ".vue", "/", "artifact", "stdout", "stderr", "response", "request",
        ],
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
    fn sentinel_branch_creates_new_intent_with_parent_link() {
        let existing = vec![SentinelIntentState {
            intent_id: "intent-1".to_string(),
            goal: "fix login timeout in api gateway".to_string(),
            confidence: 0.9,
            status: SentinelIntentStatus::Active,
            ..SentinelIntentState::default()
        }];
        let intent = analyze_intent("另外做一个 login timeout 的分支验证", &existing);
        assert_eq!(intent.relation, SentinelIntentRelation::Branch);
        assert_ne!(intent.intent_id, "intent-1");
        assert_eq!(intent.parent_intent_id.as_deref(), Some("intent-1"));
        assert_eq!(intent.last_transition, SentinelIntentTransition::Branched);
    }

    #[test]
    fn update_intent_registry_suspends_previous_active_intent_on_resume() {
        let now_ms = 1_700_000_000_000_i64;
        let mut intents = vec![
            SentinelIntentState {
                intent_id: "intent-a".to_string(),
                goal: "fix login timeout".to_string(),
                status: SentinelIntentStatus::Suspended,
                updated_at_ms: now_ms - 20,
                ..SentinelIntentState::default()
            },
            SentinelIntentState {
                intent_id: "intent-b".to_string(),
                goal: "investigate weekly report export".to_string(),
                status: SentinelIntentStatus::Active,
                updated_at_ms: now_ms - 10,
                ..SentinelIntentState::default()
            },
        ];
        let resumed = SentinelIntentState {
            intent_id: "intent-a".to_string(),
            goal: "resume login timeout".to_string(),
            resumed_from_intent_id: Some("intent-a".to_string()),
            relation: SentinelIntentRelation::ResumeOldIntent,
            status: SentinelIntentStatus::Active,
            last_transition: SentinelIntentTransition::Resumed,
            updated_at_ms: now_ms,
            ..SentinelIntentState::default()
        };

        update_intent_registry(&mut intents, &resumed);
        let current = intents
            .iter()
            .find(|intent| intent.intent_id == "intent-a")
            .unwrap();
        let other = intents
            .iter()
            .find(|intent| intent.intent_id == "intent-b")
            .unwrap();
        assert_eq!(current.status, SentinelIntentStatus::Active);
        assert_eq!(current.last_transition, SentinelIntentTransition::Resumed);
        assert_eq!(other.status, SentinelIntentStatus::Suspended);
        assert_eq!(other.last_transition, SentinelIntentTransition::Suspended);
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

    #[test]
    fn sentinel_clarification_recommends_two_minute_question_timeout() {
        let intent = SentinelIntentState {
            clarification_needed: true,
            ..SentinelIntentState::default()
        };

        let clarification = build_sentinel_clarification_state(&intent);

        assert_eq!(clarification.recommended_timeout_secs, 120);
    }

    #[test]
    fn focus_compression_state_prefers_matching_intent_slices() {
        let compression = SentinelCompressionState {
            version: 3,
            constraints: vec![
                "A intent constraint keep login path".to_string(),
                "B intent constraint keep report path".to_string(),
            ],
            slices: vec![
                SentinelContextSlice {
                    slice_id: "slice-a".to_string(),
                    intent_id: "intent-a".to_string(),
                    source_type: "assistant".to_string(),
                    source_ref: "group-1".to_string(),
                    relevance_score: 2.8,
                    risk_level: "medium".to_string(),
                    keep_mode: "compress".to_string(),
                    content: "login timeout still fails on api gateway".to_string(),
                },
                SentinelContextSlice {
                    slice_id: "slice-b".to_string(),
                    intent_id: "intent-b".to_string(),
                    source_type: "assistant".to_string(),
                    source_ref: "group-2".to_string(),
                    relevance_score: 2.6,
                    risk_level: "medium".to_string(),
                    keep_mode: "compress".to_string(),
                    content: "weekly report formatting issue in export panel".to_string(),
                },
            ],
            ..SentinelCompressionState::default()
        };
        let intent = SentinelIntentState {
            intent_id: "intent-a".to_string(),
            goal: "continue fixing login timeout in api gateway".to_string(),
            focus_objects: vec!["login".to_string(), "gateway".to_string()],
            continuation_of: Some("intent-a".to_string()),
            ..SentinelIntentState::default()
        };

        let focused = focus_compression_state_for_intent(&compression, &intent);
        assert_eq!(focused.slices.len(), 1);
        assert_eq!(focused.slices[0].intent_id, "intent-a");
        assert!(focused.summary_text.contains("login timeout"));
    }

    #[test]
    fn restore_pinned_context_for_intent_ignores_unrelated_slices() {
        let mut pinned = SentinelPinnedContext::default();
        let compression = SentinelCompressionState {
            slices: vec![
                SentinelContextSlice {
                    slice_id: "slice-a".to_string(),
                    intent_id: "intent-a".to_string(),
                    source_type: "tool".to_string(),
                    source_ref: "history-a".to_string(),
                    relevance_score: 3.0,
                    risk_level: "high".to_string(),
                    keep_mode: "archive".to_string(),
                    content: "/tmp/login.log showed timeout failure".to_string(),
                },
                SentinelContextSlice {
                    slice_id: "slice-b".to_string(),
                    intent_id: "intent-b".to_string(),
                    source_type: "tool".to_string(),
                    source_ref: "history-b".to_string(),
                    relevance_score: 2.0,
                    risk_level: "low".to_string(),
                    keep_mode: "compress".to_string(),
                    content: "/tmp/report.csv export looked fine".to_string(),
                },
            ],
            constraints: vec![
                "keep login path intact".to_string(),
                "preserve report export".to_string(),
            ],
            ..SentinelCompressionState::default()
        };
        let intent = SentinelIntentState {
            intent_id: "intent-a".to_string(),
            goal: "resume login timeout investigation".to_string(),
            focus_objects: vec!["login".to_string()],
            ..SentinelIntentState::default()
        };

        restore_pinned_context_for_intent(&mut pinned, &compression, &intent);
        assert!(pinned
            .must_keep_facts
            .iter()
            .any(|item| item.contains("login.log")));
        assert!(!pinned
            .must_keep_facts
            .iter()
            .any(|item| item.contains("report.csv")));
    }
}
