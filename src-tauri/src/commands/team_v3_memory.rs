use std::collections::HashSet;

use anyhow::Result;
use sentinel_db::database_service::connection_manager::DatabasePool;
use serde_json::{json, Value};

use super::team_v3_artifact_store::TeamV3ArtifactFileRef;
use super::team_v3_commands::{
    append_team_v3_blackboard_entry, blackboard_revision_from_metadata_value,
    clip_to_sentence_boundary, collapse_whitespace, list_team_v3_blackboard_entries,
    normalize_fact_for_prompt, TeamV3BlackboardEntry,
};

#[derive(Debug, Clone, Default)]
pub(crate) struct TeamV3CheckpointFacts {
    pub(crate) task_key: String,
    pub(crate) title: String,
    pub(crate) conclusion: Option<String>,
    pub(crate) evidence: Option<String>,
    pub(crate) risk: Option<String>,
    pub(crate) next_step: Option<String>,
    pub(crate) highlights: Vec<String>,
}

#[derive(Debug, Clone)]
pub(crate) struct TeamV3TaskCheckpointPayload {
    pub(crate) content: String,
    pub(crate) facts: Value,
}

#[derive(Debug, Clone)]
pub(crate) struct TeamV3StructuredFactPayload {
    pub(crate) task_key: String,
    pub(crate) task_title: String,
    pub(crate) content: String,
    pub(crate) facts: Value,
    pub(crate) tags: Vec<String>,
}

pub(crate) fn parse_checkpoint_content_fields(
    content: &str,
) -> (Option<String>, Option<String>, Vec<String>) {
    let mut task_key = None;
    let mut title = None;
    let mut points = Vec::new();
    for raw_line in content.lines() {
        let line = raw_line.trim();
        if let Some(value) = line.strip_prefix("task_key=") {
            let normalized = collapse_whitespace(value);
            if !normalized.is_empty() {
                task_key = Some(normalized);
            }
            continue;
        }
        if let Some(value) = line.strip_prefix("title=") {
            let normalized = collapse_whitespace(value);
            if !normalized.is_empty() {
                title = Some(normalized);
            }
            continue;
        }
        if let Some(value) = line.strip_prefix("- ") {
            let normalized = collapse_whitespace(value);
            if !normalized.is_empty() {
                points.push(normalized);
            }
        }
    }
    (task_key, title, points)
}

pub(crate) fn infer_checkpoint_structured_fields(
    points: &[String],
) -> (
    Option<String>,
    Option<String>,
    Option<String>,
    Option<String>,
) {
    let mut conclusion = None;
    let mut evidence = None;
    let mut risk = None;
    let mut next_step = None;
    for point in points {
        let lower = point.to_lowercase();
        let is_conclusion = point.contains("结论")
            || point.contains("总结")
            || lower.contains("conclusion")
            || lower.contains("summary");
        let is_evidence =
            point.contains("依据") || point.contains("证据") || lower.contains("evidence");
        let is_risk = point.contains("风险")
            || point.contains("漏洞")
            || point.contains("隐患")
            || lower.contains("risk")
            || lower.contains("impact");
        let is_next = point.contains("下一步")
            || point.contains("建议")
            || point.contains("行动")
            || lower.contains("next")
            || lower.contains("recommend");
        if conclusion.is_none() && is_conclusion {
            conclusion = Some(point.clone());
            continue;
        }
        if evidence.is_none() && is_evidence {
            evidence = Some(point.clone());
            continue;
        }
        if risk.is_none() && is_risk {
            risk = Some(point.clone());
            continue;
        }
        if next_step.is_none() && is_next {
            next_step = Some(point.clone());
            continue;
        }
    }
    (conclusion, evidence, risk, next_step)
}

pub(crate) fn is_placeholder_fact(input: &str) -> bool {
    let normalized = collapse_whitespace(input)
        .trim_matches(|ch: char| matches!(ch, ':' | '：' | '-' | '|' | '*' | '#' | '。'))
        .to_string();
    matches!(
        normalized.as_str(),
        "结论"
            | "依据"
            | "证据"
            | "风险"
            | "下一步"
            | "建议"
            | "行动"
            | "Conclusion"
            | "Evidence"
            | "Risk"
            | "Next"
            | "Recommendation"
    )
}

pub(crate) fn sanitize_structured_fact_value(value: Option<String>) -> Option<String> {
    value
        .map(|item| collapse_whitespace(item.as_str()))
        .filter(|item| !item.is_empty())
        .filter(|item| !is_placeholder_fact(item.as_str()))
}

pub(crate) fn extract_checkpoint_facts(entry: &TeamV3BlackboardEntry) -> TeamV3CheckpointFacts {
    let (content_task_key, content_title, content_points) =
        parse_checkpoint_content_fields(entry.content.as_str());
    let metadata_task_key = entry
        .metadata
        .get("task_key")
        .and_then(Value::as_str)
        .map(collapse_whitespace)
        .filter(|value| !value.is_empty());
    let metadata_title = entry
        .metadata
        .get("task_title")
        .and_then(Value::as_str)
        .map(collapse_whitespace)
        .filter(|value| !value.is_empty());
    let metadata_facts = entry.metadata.get("facts");

    let metadata_points = metadata_facts
        .and_then(|facts| facts.get("highlights"))
        .and_then(Value::as_array)
        .map(|items| {
            items
                .iter()
                .filter_map(Value::as_str)
                .map(collapse_whitespace)
                .filter(|value| !value.is_empty())
                .collect::<Vec<_>>()
        })
        .unwrap_or_default();

    let points = if metadata_points.is_empty() {
        content_points
    } else {
        metadata_points
    };

    let mut conclusion = sanitize_structured_fact_value(
        metadata_facts
            .and_then(|facts| facts.get("conclusion"))
            .and_then(Value::as_str)
            .map(collapse_whitespace)
            .filter(|value| !value.is_empty()),
    );
    let mut evidence = sanitize_structured_fact_value(
        metadata_facts
            .and_then(|facts| facts.get("evidence"))
            .and_then(Value::as_str)
            .map(collapse_whitespace)
            .filter(|value| !value.is_empty()),
    );
    let mut risk = sanitize_structured_fact_value(
        metadata_facts
            .and_then(|facts| facts.get("risk"))
            .and_then(Value::as_str)
            .map(collapse_whitespace)
            .filter(|value| !value.is_empty()),
    );
    let mut next_step = sanitize_structured_fact_value(
        metadata_facts
            .and_then(|facts| facts.get("next_step"))
            .and_then(Value::as_str)
            .map(collapse_whitespace)
            .filter(|value| !value.is_empty()),
    );

    let (inferred_conclusion, inferred_evidence, inferred_risk, inferred_next_step) =
        infer_checkpoint_structured_fields(&points);
    if conclusion.is_none() {
        conclusion = sanitize_structured_fact_value(inferred_conclusion);
    }
    if evidence.is_none() {
        evidence = sanitize_structured_fact_value(inferred_evidence);
    }
    if risk.is_none() {
        risk = sanitize_structured_fact_value(inferred_risk);
    }
    if next_step.is_none() {
        next_step = sanitize_structured_fact_value(inferred_next_step);
    }

    let mut highlights = points
        .iter()
        .filter(|point| {
            Some(*point) != conclusion.as_ref()
                && Some(*point) != evidence.as_ref()
                && Some(*point) != risk.as_ref()
                && Some(*point) != next_step.as_ref()
        })
        .take(4)
        .cloned()
        .collect::<Vec<_>>();
    if highlights.is_empty() && conclusion.is_none() && evidence.is_none() && risk.is_none() {
        highlights = points.into_iter().take(4).collect::<Vec<_>>();
    }

    TeamV3CheckpointFacts {
        task_key: metadata_task_key
            .or(content_task_key)
            .unwrap_or_else(|| "-".to_string()),
        title: metadata_title
            .or(content_title)
            .unwrap_or_else(|| "未命名任务".to_string()),
        conclusion,
        evidence,
        risk,
        next_step,
        highlights,
    }
}

fn normalize_checkpoint_line(line: &str) -> Option<String> {
    let trimmed = line
        .trim()
        .trim_start_matches(|ch: char| {
            matches!(
                ch,
                '-' | '*' | '#' | '>' | '•' | '·' | '1'..='9' | '0' | '.' | '、' | ')' | '('
            )
        })
        .trim();
    if trimmed.is_empty() {
        return None;
    }
    let collapsed = collapse_whitespace(trimmed);
    if collapsed.is_empty() {
        None
    } else {
        Some(collapsed)
    }
}

fn collect_checkpoint_points(
    output: &str,
    max_points: usize,
    max_chars_per_point: usize,
) -> Vec<String> {
    let priority_terms = [
        "结论",
        "依据",
        "风险",
        "下一步",
        "建议",
        "行动",
        "结论:",
        "Conclusion",
        "Evidence",
        "Risk",
        "Next",
        "Recommendation",
    ];
    let mut seen = HashSet::new();
    let mut prioritized = Vec::new();
    let mut fallback = Vec::new();
    for raw_line in output.lines() {
        let Some(line) = normalize_checkpoint_line(raw_line) else {
            continue;
        };
        if !seen.insert(line.clone()) {
            continue;
        }
        let Some(clipped) = normalize_fact_for_prompt(line.as_str(), max_chars_per_point) else {
            continue;
        };
        let lower = clipped.to_lowercase();
        let has_priority = priority_terms.iter().any(|term| {
            if term.chars().all(|ch| ch.is_ascii()) {
                lower.contains(&term.to_lowercase())
            } else {
                clipped.contains(term)
            }
        });
        if has_priority {
            prioritized.push(clipped);
        } else {
            fallback.push(clipped);
        }
    }
    let mut points = Vec::new();
    points.extend(prioritized.into_iter().take(max_points));
    if points.len() < max_points {
        points.extend(fallback.into_iter().take(max_points - points.len()));
    }
    if points.is_empty() {
        let fallback_point = clip_to_sentence_boundary(output, max_chars_per_point)
            .unwrap_or_else(|| "任务完成，待进一步提炼。".to_string());
        points.push(fallback_point);
    }
    points
}

fn build_task_working_memory_content(task_key: &str, output: &str) -> String {
    let first_point = collect_checkpoint_points(output, 1, 220)
        .into_iter()
        .next()
        .unwrap_or_else(|| "任务完成，待进一步提炼。".to_string());
    format!("task_key={}: {}", task_key, first_point)
}

fn parse_fact_fields_from_value(
    facts: &Value,
) -> (
    Option<String>,
    Option<String>,
    Option<String>,
    Option<String>,
    Vec<String>,
) {
    let conclusion = sanitize_structured_fact_value(
        facts
            .get("conclusion")
            .and_then(Value::as_str)
            .map(|value| value.to_string()),
    );
    let evidence = sanitize_structured_fact_value(
        facts
            .get("evidence")
            .and_then(Value::as_str)
            .map(|value| value.to_string()),
    );
    let risk = sanitize_structured_fact_value(
        facts
            .get("risk")
            .and_then(Value::as_str)
            .map(|value| value.to_string()),
    );
    let next_step = sanitize_structured_fact_value(
        facts
            .get("next_step")
            .and_then(Value::as_str)
            .map(|value| value.to_string()),
    );
    let highlights = facts
        .get("highlights")
        .and_then(Value::as_array)
        .map(|items| {
            items
                .iter()
                .filter_map(Value::as_str)
                .map(|value| collapse_whitespace(value))
                .filter(|value| !value.is_empty())
                .filter(|value| !is_placeholder_fact(value.as_str()))
                .take(5)
                .collect::<Vec<_>>()
        })
        .unwrap_or_default();
    (conclusion, evidence, risk, next_step, highlights)
}

fn derive_security_tags_from_texts(texts: &[&str]) -> Vec<String> {
    let merged = texts
        .iter()
        .map(|value| value.to_lowercase())
        .collect::<Vec<_>>()
        .join(" ");
    let mut tags = Vec::new();
    if merged.contains("sql") || merged.contains("注入") {
        tags.push("sql-injection");
    }
    if merged.contains("rce") || merged.contains("远程代码执行") {
        tags.push("rce");
    }
    if merged.contains("未授权")
        || merged.contains("越权")
        || merged.contains("unauthor")
        || merged.contains("privilege")
    {
        tags.push("unauthorized-access");
    }
    if merged.contains("source-sink") || merged.contains("source sink") {
        tags.push("source-sink");
    }
    if merged.contains("csrf") {
        tags.push("csrf");
    }
    if tags.is_empty() {
        tags.push("general");
    }
    tags.into_iter().map(|value| value.to_string()).collect()
}

fn build_structured_fact_content(
    task_key: &str,
    task_title: &str,
    facts: &Value,
    fallback: &str,
) -> Option<String> {
    let (conclusion, evidence, risk, next_step, highlights) = parse_fact_fields_from_value(facts);
    if conclusion.is_none()
        && evidence.is_none()
        && risk.is_none()
        && next_step.is_none()
        && highlights.is_empty()
    {
        let fallback_line = normalize_fact_for_prompt(fallback, 220)?;
        if is_placeholder_fact(fallback_line.as_str()) {
            return None;
        }
        return Some(format!(
            "task_key={}\ntitle={}\nkey_points:\n- {}",
            task_key,
            collapse_whitespace(task_title),
            fallback_line
        ));
    }

    let mut lines = vec![
        format!("task_key={}", task_key),
        format!("title={}", collapse_whitespace(task_title)),
        "key_points:".to_string(),
    ];
    if let Some(value) = conclusion {
        lines.push(format!("- 结论: {}", value));
    }
    if let Some(value) = evidence {
        lines.push(format!("- 依据: {}", value));
    }
    if let Some(value) = risk {
        lines.push(format!("- 风险: {}", value));
    }
    if let Some(value) = next_step {
        lines.push(format!("- 下一步: {}", value));
    }
    lines.extend(highlights.into_iter().map(|value| format!("- {}", value)));
    Some(lines.join("\n"))
}

fn build_structured_fact_payload(
    task_key: &str,
    task_title: &str,
    facts: &Value,
    fallback: &str,
) -> Option<TeamV3StructuredFactPayload> {
    let content = build_structured_fact_content(task_key, task_title, facts, fallback)?;
    let (conclusion, evidence, risk, next_step, highlights) = parse_fact_fields_from_value(facts);
    let facts_value = json!({
        "conclusion": conclusion,
        "evidence": evidence,
        "risk": risk,
        "next_step": next_step,
        "highlights": highlights,
    });
    let tags = derive_security_tags_from_texts(&[content.as_str(), fallback]);
    Some(TeamV3StructuredFactPayload {
        task_key: task_key.to_string(),
        task_title: collapse_whitespace(task_title),
        content,
        facts: facts_value,
        tags,
    })
}

pub(crate) fn build_task_checkpoint_payload(
    task_key: &str,
    task_title: &str,
    output: &str,
) -> TeamV3TaskCheckpointPayload {
    let points = collect_checkpoint_points(output, 6, 220);
    let (raw_conclusion, raw_evidence, raw_risk, raw_next_step) =
        infer_checkpoint_structured_fields(&points);
    let conclusion = sanitize_structured_fact_value(raw_conclusion);
    let evidence = sanitize_structured_fact_value(raw_evidence);
    let risk = sanitize_structured_fact_value(raw_risk);
    let next_step = sanitize_structured_fact_value(raw_next_step);
    let highlights = points
        .iter()
        .filter(|point| {
            Some(*point) != conclusion.as_ref()
                && Some(*point) != evidence.as_ref()
                && Some(*point) != risk.as_ref()
                && Some(*point) != next_step.as_ref()
                && !is_placeholder_fact(point.as_str())
        })
        .take(4)
        .cloned()
        .collect::<Vec<_>>();
    let mut lines = vec![
        format!("task_key={}", task_key),
        format!("title={}", collapse_whitespace(task_title)),
        "key_points:".to_string(),
    ];
    lines.extend(points.iter().map(|point| format!("- {}", point)));
    TeamV3TaskCheckpointPayload {
        content: lines.join("\n"),
        facts: json!({
            "conclusion": conclusion,
            "evidence": evidence,
            "risk": risk,
            "next_step": next_step,
            "highlights": highlights,
        }),
    }
}

pub(crate) fn build_task_artifact_summary(output: &str) -> String {
    let points = collect_checkpoint_points(output, 4, 220);
    if points.is_empty() {
        return "未提取到摘要，请直接阅读 artifact 文件。".to_string();
    }
    points
        .into_iter()
        .map(|point| format!("- {}", point))
        .collect::<Vec<_>>()
        .join("\n")
}

pub(crate) fn build_task_artifact_ref_content(summary: &str, path: &str, bytes: usize) -> String {
    format!(
        "任务产出较长，已落地为 artifact。\n摘要：\n{}\n\nartifact_path: {}\nartifact_bytes: {}",
        summary, path, bytes
    )
}

pub(crate) fn build_structured_fact_payload_from_checkpoint_entry(
    checkpoint: &TeamV3BlackboardEntry,
) -> Option<TeamV3StructuredFactPayload> {
    let facts = extract_checkpoint_facts(checkpoint);
    let (_, _, point_candidates) = parse_checkpoint_content_fields(checkpoint.content.as_str());
    let fallback = point_candidates
        .into_iter()
        .map(|value| collapse_whitespace(value.as_str()))
        .find(|value| !value.is_empty() && !is_placeholder_fact(value.as_str()))
        .unwrap_or_default();
    let facts_value = json!({
        "conclusion": facts.conclusion,
        "evidence": facts.evidence,
        "risk": facts.risk,
        "next_step": facts.next_step,
        "highlights": facts.highlights,
    });
    build_structured_fact_payload(
        facts.task_key.as_str(),
        facts.title.as_str(),
        &facts_value,
        fallback.as_str(),
    )
}

pub(crate) async fn backfill_team_v3_structured_memory_from_checkpoints(
    runtime_pool: &DatabasePool,
    session_id: &str,
    scan_limit: i64,
) -> Result<usize> {
    let entries = list_team_v3_blackboard_entries(runtime_pool, session_id, scan_limit).await?;
    if entries.is_empty() {
        return Ok(0);
    }

    let mut existing_source_checkpoint_ids = entries
        .iter()
        .filter(|entry| entry.entry_type == "structured_fact")
        .filter_map(|entry| {
            entry
                .metadata
                .get("source_checkpoint_entry_id")
                .and_then(Value::as_str)
                .map(str::trim)
                .filter(|value| !value.is_empty())
                .map(|value| value.to_string())
        })
        .collect::<HashSet<_>>();
    let mut existing_structured_task_keys = entries
        .iter()
        .filter(|entry| entry.entry_type == "structured_fact")
        .filter_map(|entry| {
            let source_type = entry
                .metadata
                .get("source_type")
                .and_then(Value::as_str)
                .map(str::trim)
                .unwrap_or_default();
            if source_type != "task_output" {
                return None;
            }
            let task_key = entry
                .metadata
                .get("task_key")
                .and_then(Value::as_str)
                .map(collapse_whitespace)
                .filter(|value| !value.is_empty())?;
            let task_id = entry
                .task_id
                .as_deref()
                .map(str::trim)
                .filter(|value| !value.is_empty())
                .unwrap_or("-");
            Some(format!("{}::{}", task_id, task_key))
        })
        .collect::<HashSet<_>>();

    let mut appended = 0usize;
    for checkpoint in entries
        .iter()
        .rev()
        .filter(|entry| entry.entry_type == "checkpoint")
    {
        if existing_source_checkpoint_ids.contains(checkpoint.id.as_str()) {
            continue;
        }
        let Some(structured) = build_structured_fact_payload_from_checkpoint_entry(checkpoint)
        else {
            continue;
        };
        let task_id_for_key = checkpoint
            .task_id
            .as_deref()
            .map(str::trim)
            .filter(|value| !value.is_empty())
            .unwrap_or("-");
        let task_key = structured.task_key.clone();
        let dedupe_key = format!("{}::{}", task_id_for_key, task_key);
        if existing_structured_task_keys.contains(dedupe_key.as_str()) {
            continue;
        }
        let source_revision =
            blackboard_revision_from_metadata_value(&checkpoint.metadata).unwrap_or(0);
        let structured_meta = json!({
            "task_key": task_key,
            "task_title": structured.task_title,
            "source_type": "checkpoint_backfill",
            "source_checkpoint_entry_id": checkpoint.id,
            "source_checkpoint_revision": source_revision,
            "facts": structured.facts,
            "tags": structured.tags,
        });
        append_team_v3_blackboard_entry(
            runtime_pool,
            session_id,
            checkpoint.task_id.as_deref(),
            checkpoint.agent_id.as_deref(),
            "structured_fact",
            structured.content.as_str(),
            Some(&structured_meta),
        )
        .await?;
        existing_source_checkpoint_ids.insert(checkpoint.id.clone());
        existing_structured_task_keys.insert(dedupe_key);
        appended += 1;
    }
    Ok(appended)
}

pub(crate) async fn append_team_v3_task_memory_layers(
    runtime_pool: &DatabasePool,
    session_id: &str,
    task_id: &str,
    member_id: &str,
    task_key: &str,
    task_title: &str,
    output: &str,
    dependency_task_ids: &[String],
    dependency_task_keys: &[String],
    artifact_ref: Option<&TeamV3ArtifactFileRef>,
) -> Result<()> {
    let working_memory = build_task_working_memory_content(task_key, output);
    let working_meta = json!({
        "task_key": task_key,
        "source_type": "task_output",
    });
    append_team_v3_blackboard_entry(
        runtime_pool,
        session_id,
        Some(task_id),
        Some(member_id),
        "working_memory",
        working_memory.as_str(),
        Some(&working_meta),
    )
    .await?;

    let checkpoint = build_task_checkpoint_payload(task_key, task_title, output);
    let checkpoint_meta = json!({
        "task_key": task_key,
        "task_title": task_title,
        "source_type": "task_output",
        "facts": checkpoint.facts,
    });
    append_team_v3_blackboard_entry(
        runtime_pool,
        session_id,
        Some(task_id),
        Some(member_id),
        "checkpoint",
        checkpoint.content.as_str(),
        Some(&checkpoint_meta),
    )
    .await?;
    if let Some(structured) =
        build_structured_fact_payload(task_key, task_title, &checkpoint.facts, output)
    {
        let mut structured_meta = json!({
            "task_key": task_key,
            "task_title": task_title,
            "source_type": "task_output",
            "facts": structured.facts,
            "depends_on_task_ids": dependency_task_ids,
            "depends_on_task_keys": dependency_task_keys,
            "tags": structured.tags,
        });
        if let Some(artifact) = artifact_ref {
            if let Some(obj) = structured_meta.as_object_mut() {
                obj.insert(
                    "artifact".to_string(),
                    json!({
                        "path": artifact.path.as_str(),
                        "bytes": artifact.bytes,
                        "host_path": artifact.host_path.as_deref(),
                        "container_path": artifact.container_path.as_deref(),
                    }),
                );
            }
        }
        append_team_v3_blackboard_entry(
            runtime_pool,
            session_id,
            Some(task_id),
            Some(member_id),
            "structured_fact",
            structured.content.as_str(),
            Some(&structured_meta),
        )
        .await?;
    }
    Ok(())
}
