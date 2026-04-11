use std::collections::HashSet;

use serde_json::Value;

use super::team_v3_commands::{
    blackboard_revision_from_metadata_value, collapse_whitespace, normalize_fact_for_prompt,
    TeamV3BlackboardEntry, TeamV3Task,
};
use super::team_v3_memory::extract_checkpoint_facts;

#[derive(Debug, Clone, Default)]
pub(crate) struct TeamV3PromptQuery {
    pub(crate) goal: String,
    pub(crate) user_input: String,
    pub(crate) task_title: String,
    pub(crate) task_instruction: String,
    pub(crate) dependency_task_ids: Vec<String>,
    pub(crate) dependency_task_keys: Vec<String>,
}

impl TeamV3PromptQuery {
    pub(crate) fn for_planner(goal: &str, user_input: &str) -> Self {
        Self {
            goal: collapse_whitespace(goal),
            user_input: collapse_whitespace(user_input),
            task_title: String::new(),
            task_instruction: String::new(),
            dependency_task_ids: Vec::new(),
            dependency_task_keys: Vec::new(),
        }
    }

    pub(crate) fn for_task(
        goal: &str,
        user_input: &str,
        task: &TeamV3Task,
        dependency_task_ids: &[String],
        dependency_task_keys: &[String],
    ) -> Self {
        Self {
            goal: collapse_whitespace(goal),
            user_input: collapse_whitespace(user_input),
            task_title: collapse_whitespace(task.title.as_str()),
            task_instruction: collapse_whitespace(task.instruction.as_str()),
            dependency_task_ids: dependency_task_ids.to_vec(),
            dependency_task_keys: dependency_task_keys.to_vec(),
        }
    }
}

#[derive(Debug, Clone, Default)]
pub(crate) struct TeamV3BlackboardSectionDiagnostics {
    pub(crate) total: usize,
    pub(crate) selected: usize,
}

impl TeamV3BlackboardSectionDiagnostics {
    pub(crate) fn dropped(&self) -> usize {
        self.total.saturating_sub(self.selected)
    }
}

#[derive(Debug, Clone, Default)]
pub(crate) struct TeamV3BlackboardContextDiagnostics {
    pub(crate) mode: String,
    pub(crate) query_terms: usize,
    pub(crate) structured_memory: TeamV3BlackboardSectionDiagnostics,
    pub(crate) task_outputs: TeamV3BlackboardSectionDiagnostics,
    pub(crate) raw_events: TeamV3BlackboardSectionDiagnostics,
    pub(crate) raw_log: TeamV3BlackboardSectionDiagnostics,
    pub(crate) artifacts: TeamV3BlackboardSectionDiagnostics,
    pub(crate) checkpoints: TeamV3BlackboardSectionDiagnostics,
    pub(crate) context_chars: usize,
}

#[derive(Debug, Clone, Default)]
pub(crate) struct TeamV3BlackboardContextBuildResult {
    pub(crate) context: String,
    pub(crate) diagnostics: TeamV3BlackboardContextDiagnostics,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum TeamV3BlackboardContextMode {
    Standard,
    CheckpointOnly,
}

#[derive(Default)]
pub(crate) struct TeamV3BlackboardLayers<'a> {
    pub(crate) structured_memory: Vec<&'a TeamV3BlackboardEntry>,
    pub(crate) task_outputs: Vec<&'a TeamV3BlackboardEntry>,
    pub(crate) raw_events: Vec<&'a TeamV3BlackboardEntry>,
    pub(crate) raw_log: Vec<&'a TeamV3BlackboardEntry>,
    pub(crate) checkpoints: Vec<&'a TeamV3BlackboardEntry>,
    pub(crate) artifacts: Vec<&'a TeamV3BlackboardEntry>,
}

pub(crate) fn split_blackboard_layers<'a>(
    entries: &'a [TeamV3BlackboardEntry],
) -> TeamV3BlackboardLayers<'a> {
    let mut layers = TeamV3BlackboardLayers::default();
    for entry in entries {
        match entry.entry_type.as_str() {
            "structured_fact" => layers.structured_memory.push(entry),
            "task_output" => layers.task_outputs.push(entry),
            "working_memory" => layers.raw_events.push(entry),
            "checkpoint" => layers.checkpoints.push(entry),
            "artifact_ref" => layers.artifacts.push(entry),
            "goal" | "plan" | "task_start" | "task_error" | "plan_fallback" => {
                layers.raw_events.push(entry)
            }
            _ => layers.raw_log.push(entry),
        }
    }
    layers
}

pub(crate) fn truncate_chars(input: &str, max_chars: usize) -> String {
    if max_chars == 0 {
        return String::new();
    }
    let total = input.chars().count();
    if total <= max_chars {
        return input.to_string();
    }
    if max_chars <= 8 {
        return input.chars().take(max_chars).collect::<String>();
    }
    let head = max_chars.saturating_mul(3) / 4;
    let tail = max_chars.saturating_sub(head + 5);
    let prefix = input.chars().take(head).collect::<String>();
    let suffix = input
        .chars()
        .rev()
        .take(tail)
        .collect::<String>()
        .chars()
        .rev()
        .collect::<String>();
    format!("{} ... {}", prefix.trim_end(), suffix.trim_start())
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

fn extract_query_terms(text: &str) -> Vec<String> {
    const MAX_TERMS: usize = 36;
    let stop_words = [
        "the", "and", "for", "with", "that", "this", "from", "into", "then", "have", "need",
        "task", "team", "用户", "针对", "进行", "继续", "然后", "主要", "关注", "所有", "一个",
    ];
    let mut terms = Vec::new();
    let mut seen = HashSet::new();
    for token in text
        .to_lowercase()
        .split(|ch: char| !ch.is_ascii_alphanumeric() && !is_cjk(ch))
    {
        let normalized = token.trim();
        if normalized.is_empty() {
            continue;
        }
        let token_len = normalized.chars().count();
        if token_len < 2 {
            continue;
        }
        if token_len < 4 && normalized.chars().all(|ch| ch.is_ascii()) {
            continue;
        }
        if stop_words.contains(&normalized) {
            continue;
        }
        if seen.insert(normalized.to_string()) {
            terms.push(normalized.to_string());
            if terms.len() >= MAX_TERMS {
                break;
            }
        }
    }
    terms
}

fn build_query_terms(query: &TeamV3PromptQuery) -> Vec<String> {
    let mut raw = Vec::new();
    if !query.goal.trim().is_empty() {
        raw.push(query.goal.as_str());
    }
    if !query.user_input.trim().is_empty() {
        raw.push(query.user_input.as_str());
    }
    if !query.task_title.trim().is_empty() {
        raw.push(query.task_title.as_str());
    }
    if !query.task_instruction.trim().is_empty() {
        raw.push(query.task_instruction.as_str());
    }
    for dep in query.dependency_task_keys.iter().take(24) {
        raw.push(dep.as_str());
    }
    extract_query_terms(raw.join(" ").as_str())
}

fn entry_search_text(entry: &TeamV3BlackboardEntry) -> String {
    let mut segments = vec![
        entry.entry_type.as_str().to_string(),
        entry.content.as_str().to_string(),
    ];
    if let Some(task_key) = entry.metadata.get("task_key").and_then(Value::as_str) {
        segments.push(task_key.to_string());
    }
    if let Some(task_title) = entry.metadata.get("task_title").and_then(Value::as_str) {
        segments.push(task_title.to_string());
    }
    if let Some(facts) = entry.metadata.get("facts").and_then(Value::as_object) {
        for value in facts.values() {
            match value {
                Value::String(text) => segments.push(text.clone()),
                Value::Array(items) => {
                    for item in items {
                        if let Some(text) = item.as_str() {
                            segments.push(text.to_string());
                        }
                    }
                }
                _ => {}
            }
        }
    }
    collapse_whitespace(segments.join(" ").as_str()).to_lowercase()
}

fn entry_base_weight(entry_type: &str) -> i64 {
    match entry_type {
        "structured_fact" => 70,
        "checkpoint" => 55,
        "artifact_ref" => 36,
        "task_output" => 30,
        "working_memory" => 26,
        "plan" => 20,
        "goal" => 12,
        _ => 8,
    }
}

fn score_blackboard_entry(
    entry: &TeamV3BlackboardEntry,
    terms: &[String],
    query: &TeamV3PromptQuery,
    recency_index: usize,
) -> i64 {
    let search_text = entry_search_text(entry);
    let mut score = entry_base_weight(entry.entry_type.as_str());
    for term in terms {
        if search_text.contains(term) {
            score += (term.chars().count().min(18) as i64) * 2;
        }
    }
    let task_id_match = entry
        .task_id
        .as_deref()
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(|value| query.dependency_task_ids.iter().any(|dep| dep == value))
        .unwrap_or(false);
    let task_key_match = entry
        .metadata
        .get("task_key")
        .and_then(Value::as_str)
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(|value| query.dependency_task_keys.iter().any(|dep| dep == value))
        .unwrap_or(false);
    if task_id_match || task_key_match {
        score += 95;
    }
    if search_text.contains("sql注入")
        || search_text.contains("rce")
        || search_text.contains("未授权")
        || search_text.contains("source-sink")
        || search_text.contains("风险")
        || search_text.contains("漏洞")
    {
        score += 22;
    }
    if search_text.contains("最终结论")
        || search_text.contains("汇总")
        || search_text.contains("清单")
        || search_text.contains("基线")
    {
        score += 14;
    }
    score + ((120usize.saturating_sub(recency_index)) as i64 / 6)
}

fn select_blackboard_entries<'a>(
    entries: &[&'a TeamV3BlackboardEntry],
    limit: usize,
    always_keep_recent: usize,
    terms: &[String],
    query: &TeamV3PromptQuery,
) -> Vec<&'a TeamV3BlackboardEntry> {
    if entries.is_empty() || limit == 0 {
        return Vec::new();
    }

    let mut selected = HashSet::new();
    for index in 0..entries.len().min(always_keep_recent) {
        selected.insert(index);
    }

    let mut scored = entries
        .iter()
        .enumerate()
        .map(|(index, entry)| (index, score_blackboard_entry(entry, terms, query, index)))
        .collect::<Vec<_>>();
    scored.sort_by(|a, b| b.1.cmp(&a.1).then_with(|| a.0.cmp(&b.0)));

    for (index, _) in scored {
        if selected.len() >= limit {
            break;
        }
        selected.insert(index);
    }

    if selected.len() < limit {
        for index in 0..entries.len() {
            if selected.len() >= limit {
                break;
            }
            selected.insert(index);
        }
    }

    let mut selected_entries = selected
        .into_iter()
        .map(|index| entries[index])
        .collect::<Vec<_>>();
    selected_entries.sort_by(|a, b| {
        let a_revision = blackboard_revision_from_metadata_value(&a.metadata).unwrap_or(0);
        let b_revision = blackboard_revision_from_metadata_value(&b.metadata).unwrap_or(0);
        a_revision
            .cmp(&b_revision)
            .then_with(|| a.created_at.cmp(&b.created_at))
    });
    selected_entries
}

fn normalize_blackboard_dedupe_content(content: &str) -> String {
    collapse_whitespace(content).to_lowercase()
}

pub(crate) fn dedupe_event_entries_for_prompt<'a>(
    entries: &[&'a TeamV3BlackboardEntry],
) -> Vec<&'a TeamV3BlackboardEntry> {
    if entries.is_empty() {
        return Vec::new();
    }
    let mut seen = HashSet::new();
    let mut deduped = Vec::new();
    for entry in entries {
        let task = entry
            .task_id
            .as_deref()
            .map(str::trim)
            .filter(|value| !value.is_empty())
            .unwrap_or("-");
        let agent = entry
            .agent_id
            .as_deref()
            .map(str::trim)
            .filter(|value| !value.is_empty())
            .unwrap_or("-");
        let signature = format!(
            "{}|{}|{}|{}",
            entry.entry_type,
            agent,
            task,
            normalize_blackboard_dedupe_content(entry.content.as_str())
        );
        if seen.insert(signature) {
            deduped.push(*entry);
        }
    }
    deduped
}

fn format_blackboard_entry(entry: &TeamV3BlackboardEntry, max_chars: usize) -> String {
    let agent = entry
        .agent_id
        .as_deref()
        .filter(|value| !value.trim().is_empty())
        .unwrap_or("unknown-agent");
    let task = entry
        .task_id
        .as_deref()
        .filter(|value| !value.trim().is_empty())
        .unwrap_or("-");
    if entry.entry_type == "artifact_ref" {
        let summary = entry
            .metadata
            .get("summary")
            .and_then(Value::as_str)
            .map(str::trim)
            .filter(|value| !value.is_empty())
            .unwrap_or(entry.content.as_str());
        let file_path = entry
            .metadata
            .get("artifact")
            .and_then(|value| value.get("path"))
            .and_then(Value::as_str)
            .map(str::trim)
            .filter(|value| !value.is_empty())
            .unwrap_or("-");
        let file_bytes = entry
            .metadata
            .get("artifact")
            .and_then(|value| value.get("bytes"))
            .and_then(Value::as_u64)
            .unwrap_or(0);
        let revision = blackboard_revision_from_metadata_value(&entry.metadata).unwrap_or(0);
        let summary_content = truncate_chars(
            collapse_whitespace(summary).as_str(),
            max_chars.saturating_sub(40),
        );
        let path_content = truncate_chars(file_path, 140);
        return format!(
            "- [{}][rev={}][agent={}][task={}][bytes={}] {} | file={}",
            entry.entry_type, revision, agent, task, file_bytes, summary_content, path_content
        );
    }
    let content = truncate_chars(
        collapse_whitespace(entry.content.as_str()).as_str(),
        max_chars,
    );
    let revision = blackboard_revision_from_metadata_value(&entry.metadata).unwrap_or(0);
    format!(
        "- [{}][rev={}][agent={}][task={}] {}",
        entry.entry_type, revision, agent, task, content
    )
}

pub(crate) fn latest_blackboard_revision(entries: &[TeamV3BlackboardEntry]) -> i64 {
    entries
        .iter()
        .filter_map(|entry| blackboard_revision_from_metadata_value(&entry.metadata))
        .max()
        .unwrap_or(0)
}

fn build_blackboard_section_with_formatter<'a, F>(
    title: &str,
    entries: &[&'a TeamV3BlackboardEntry],
    limit: usize,
    always_keep_recent: usize,
    terms: &[String],
    query: &TeamV3PromptQuery,
    formatter: F,
) -> (Option<String>, TeamV3BlackboardSectionDiagnostics)
where
    F: Fn(&TeamV3BlackboardEntry) -> String,
{
    let total = entries.len();
    if total == 0 || limit == 0 {
        return (
            None,
            TeamV3BlackboardSectionDiagnostics { total, selected: 0 },
        );
    }
    let selected = select_blackboard_entries(entries, limit, always_keep_recent, terms, query);
    if selected.is_empty() {
        return (
            None,
            TeamV3BlackboardSectionDiagnostics { total, selected: 0 },
        );
    }
    let rows = selected
        .iter()
        .map(|entry| formatter(entry))
        .collect::<Vec<_>>();
    (
        Some(format!("{}\n{}", title, rows.join("\n"))),
        TeamV3BlackboardSectionDiagnostics {
            total,
            selected: selected.len(),
        },
    )
}

fn format_checkpoint_entry_for_prompt(entry: &TeamV3BlackboardEntry) -> String {
    let revision = blackboard_revision_from_metadata_value(&entry.metadata).unwrap_or(0);
    let agent = entry
        .agent_id
        .as_deref()
        .filter(|value| !value.trim().is_empty())
        .unwrap_or("unknown-agent");
    let task = entry
        .task_id
        .as_deref()
        .filter(|value| !value.trim().is_empty())
        .unwrap_or("-");
    let facts = extract_checkpoint_facts(entry);
    let mut fields = vec![
        format!(
            "- [checkpoint][rev={}][agent={}][task={}]",
            revision, agent, task
        ),
        format!("task_key={}", facts.task_key),
        format!("title={}", facts.title),
    ];

    if let Some(value) = facts
        .conclusion
        .and_then(|value| normalize_fact_for_prompt(value.as_str(), 260))
    {
        fields.push(format!("结论={}", value));
    }
    if let Some(value) = facts
        .evidence
        .and_then(|value| normalize_fact_for_prompt(value.as_str(), 260))
    {
        fields.push(format!("依据={}", value));
    }
    if let Some(value) = facts
        .risk
        .and_then(|value| normalize_fact_for_prompt(value.as_str(), 260))
    {
        fields.push(format!("风险={}", value));
    }
    if let Some(value) = facts
        .next_step
        .and_then(|value| normalize_fact_for_prompt(value.as_str(), 260))
    {
        fields.push(format!("下一步={}", value));
    }

    if !facts.highlights.is_empty() {
        let highlights = facts
            .highlights
            .iter()
            .filter_map(|value| normalize_fact_for_prompt(value.as_str(), 220))
            .collect::<Vec<_>>();
        if !highlights.is_empty() {
            fields.push(format!("要点={}", highlights.join(" | ")));
        }
    }
    fields.join(" | ")
}

fn format_structured_memory_entry_for_prompt(entry: &TeamV3BlackboardEntry) -> String {
    let revision = blackboard_revision_from_metadata_value(&entry.metadata).unwrap_or(0);
    let agent = entry
        .agent_id
        .as_deref()
        .filter(|value| !value.trim().is_empty())
        .unwrap_or("unknown-agent");
    let task = entry
        .task_id
        .as_deref()
        .filter(|value| !value.trim().is_empty())
        .unwrap_or("-");
    let task_key = entry
        .metadata
        .get("task_key")
        .and_then(Value::as_str)
        .map(collapse_whitespace)
        .filter(|value| !value.is_empty())
        .unwrap_or_else(|| "-".to_string());
    let title = entry
        .metadata
        .get("task_title")
        .and_then(Value::as_str)
        .map(collapse_whitespace)
        .filter(|value| !value.is_empty())
        .unwrap_or_else(|| "未命名任务".to_string());
    let conclusion = entry
        .metadata
        .get("facts")
        .and_then(|value| value.get("conclusion"))
        .and_then(Value::as_str)
        .and_then(|value| normalize_fact_for_prompt(value, 260));
    let evidence = entry
        .metadata
        .get("facts")
        .and_then(|value| value.get("evidence"))
        .and_then(Value::as_str)
        .and_then(|value| normalize_fact_for_prompt(value, 260));
    let risk = entry
        .metadata
        .get("facts")
        .and_then(|value| value.get("risk"))
        .and_then(Value::as_str)
        .and_then(|value| normalize_fact_for_prompt(value, 260));
    let next_step = entry
        .metadata
        .get("facts")
        .and_then(|value| value.get("next_step"))
        .and_then(Value::as_str)
        .and_then(|value| normalize_fact_for_prompt(value, 260));
    let tags = entry
        .metadata
        .get("tags")
        .and_then(Value::as_array)
        .map(|items| {
            items
                .iter()
                .filter_map(Value::as_str)
                .map(collapse_whitespace)
                .filter(|value| !value.is_empty())
                .take(6)
                .collect::<Vec<_>>()
        })
        .unwrap_or_default();
    let artifact_path = entry
        .metadata
        .get("artifact")
        .and_then(|value| value.get("path"))
        .and_then(Value::as_str)
        .map(collapse_whitespace)
        .filter(|value| !value.is_empty());

    let mut fields = vec![
        format!(
            "- [structured_fact][rev={}][agent={}][task={}]",
            revision, agent, task
        ),
        format!("task_key={}", task_key),
        format!("title={}", title),
    ];
    if let Some(value) = conclusion {
        fields.push(format!("结论={}", value));
    }
    if let Some(value) = evidence {
        fields.push(format!("依据={}", value));
    }
    if let Some(value) = risk {
        fields.push(format!("风险={}", value));
    }
    if let Some(value) = next_step {
        fields.push(format!("下一步={}", value));
    }
    if !tags.is_empty() {
        fields.push(format!("tags={}", tags.join(",")));
    }
    if let Some(path) = artifact_path {
        fields.push(format!("artifact={}", path));
    }
    if fields.len() <= 3 {
        fields.push(format!(
            "摘要={}",
            normalize_fact_for_prompt(entry.content.as_str(), 220)
                .unwrap_or_else(|| "暂无结构化摘要".to_string())
        ));
    }
    fields.join(" | ")
}

pub(crate) fn build_blackboard_context(
    entries: &[TeamV3BlackboardEntry],
    query: &TeamV3PromptQuery,
) -> TeamV3BlackboardContextBuildResult {
    build_blackboard_context_with_mode(entries, TeamV3BlackboardContextMode::Standard, query)
}

pub(crate) fn build_blackboard_checkpoint_context(
    entries: &[TeamV3BlackboardEntry],
    query: &TeamV3PromptQuery,
) -> TeamV3BlackboardContextBuildResult {
    build_blackboard_context_with_mode(entries, TeamV3BlackboardContextMode::CheckpointOnly, query)
}

fn build_blackboard_context_with_mode(
    entries: &[TeamV3BlackboardEntry],
    mode: TeamV3BlackboardContextMode,
    query: &TeamV3PromptQuery,
) -> TeamV3BlackboardContextBuildResult {
    let mut diagnostics = TeamV3BlackboardContextDiagnostics {
        mode: if mode == TeamV3BlackboardContextMode::CheckpointOnly {
            "checkpoint_only".to_string()
        } else {
            "standard".to_string()
        },
        ..TeamV3BlackboardContextDiagnostics::default()
    };
    if entries.is_empty() {
        return TeamV3BlackboardContextBuildResult {
            context: String::new(),
            diagnostics,
        };
    }

    let query_terms = build_query_terms(query);
    diagnostics.query_terms = query_terms.len();
    let layers = split_blackboard_layers(entries);
    let mut sections: Vec<String> = Vec::new();

    let (structured, structured_diag) = build_blackboard_section_with_formatter(
        "Structured Memory（结构化记忆）:",
        &layers.structured_memory,
        if mode == TeamV3BlackboardContextMode::CheckpointOnly {
            18
        } else {
            14
        },
        if mode == TeamV3BlackboardContextMode::CheckpointOnly {
            8
        } else {
            5
        },
        &query_terms,
        query,
        format_structured_memory_entry_for_prompt,
    );
    diagnostics.structured_memory = structured_diag;
    if let Some(structured) = structured {
        sections.push(structured);
    }

    let (task_outputs, task_output_diag) = build_blackboard_section_with_formatter(
        "Task Output Evidence（关键执行证据）:",
        &layers.task_outputs,
        if mode == TeamV3BlackboardContextMode::CheckpointOnly {
            5
        } else {
            8
        },
        2,
        &query_terms,
        query,
        |entry| format_blackboard_entry(entry, 320),
    );
    diagnostics.task_outputs = task_output_diag;
    if let Some(task_outputs) = task_outputs {
        sections.push(task_outputs);
    }

    let (artifacts, artifacts_diag) = build_blackboard_section_with_formatter(
        "Artifacts（关键产物摘要）:",
        &layers.artifacts,
        if mode == TeamV3BlackboardContextMode::CheckpointOnly {
            6
        } else {
            8
        },
        2,
        &query_terms,
        query,
        |entry| format_blackboard_entry(entry, 280),
    );
    diagnostics.artifacts = artifacts_diag;
    if let Some(artifacts) = artifacts {
        sections.push(artifacts);
    }

    let deduped_events = dedupe_event_entries_for_prompt(&layers.raw_events);
    if deduped_events.len() < layers.raw_events.len() {
        tracing::info!(
            "Team V3 prompt event dedupe applied: total={} deduped={}",
            layers.raw_events.len(),
            deduped_events.len()
        );
    }
    let (events, events_diag) = build_blackboard_section_with_formatter(
        "Team Events（关键事件）:",
        &deduped_events,
        6,
        2,
        &query_terms,
        query,
        |entry| format_blackboard_entry(entry, 220),
    );
    diagnostics.raw_events = events_diag;
    if let Some(events) = events {
        sections.push(events);
    }

    if mode == TeamV3BlackboardContextMode::Standard {
        let (raw, raw_diag) = build_blackboard_section_with_formatter(
            "Raw Log（全量事件节选）:",
            &layers.raw_log,
            4,
            1,
            &query_terms,
            query,
            |entry| format_blackboard_entry(entry, 220),
        );
        diagnostics.raw_log = raw_diag;
        if let Some(raw) = raw {
            sections.push(raw);
        }
    }

    let (checkpoints, checkpoints_diag) = build_blackboard_section_with_formatter(
        "Checkpoint（阶段总结）:",
        &layers.checkpoints,
        if mode == TeamV3BlackboardContextMode::CheckpointOnly {
            16
        } else {
            12
        },
        if mode == TeamV3BlackboardContextMode::CheckpointOnly {
            6
        } else {
            3
        },
        &query_terms,
        query,
        format_checkpoint_entry_for_prompt,
    );
    diagnostics.checkpoints = checkpoints_diag;
    if diagnostics.structured_memory.selected == 0 {
        if let Some(checkpoints) = checkpoints {
            sections.push(checkpoints);
        } else if mode == TeamV3BlackboardContextMode::CheckpointOnly {
            sections.push(
                "Checkpoint（阶段总结）:\n- 暂无 checkpoint，可基于已知依赖先产出汇总草稿。"
                    .to_string(),
            );
        }
    }

    let context = if sections.is_empty() {
        String::new()
    } else {
        format!("Team 黑板（分层共享）:\n{}", sections.join("\n\n"))
    };
    diagnostics.context_chars = context.chars().count();

    TeamV3BlackboardContextBuildResult {
        context,
        diagnostics,
    }
}
