use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};

use crate::agents::RetrievedMemoryItem;
use sentinel_db::core::models::database::{DurableMemoryProjectionState, DurableMemoryRecord};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct MemoryTraceCount {
    pub label: String,
    pub count: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct MemoryRetrievalTrace {
    pub query_preview: String,
    pub requested_top_k: usize,
    pub hit_count: usize,
    pub used_canonical_fallback: bool,
    pub include_reflection: bool,
    pub source_breakdown: Vec<MemoryTraceCount>,
    pub kind_breakdown: Vec<MemoryTraceCount>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DurableMemoryDiagnosticsItem {
    pub record: DurableMemoryRecord,
    pub projection: Option<DurableMemoryProjectionState>,
    pub retrievable_projection_ready: bool,
    pub projection_issue: bool,
}

impl DurableMemoryDiagnosticsItem {
    pub fn from_parts(
        record: DurableMemoryRecord,
        projection: Option<DurableMemoryProjectionState>,
    ) -> Self {
        let retrievable_projection_ready = projection
            .as_ref()
            .map(|state| state.lexical_indexed || state.vector_indexed)
            .unwrap_or(false);
        let projection_issue = projection
            .as_ref()
            .map(|state| !retrievable_projection_ready || !state.skill_projected)
            .unwrap_or(true);

        Self {
            record,
            projection,
            retrievable_projection_ready,
            projection_issue,
        }
    }
}

pub fn build_memory_retrieval_trace(
    query: &str,
    requested_top_k: usize,
    hits: &[RetrievedMemoryItem],
    used_canonical_fallback: bool,
    include_reflection: bool,
) -> MemoryRetrievalTrace {
    MemoryRetrievalTrace {
        query_preview: truncate_for_trace(query, 160),
        requested_top_k,
        hit_count: hits.len(),
        used_canonical_fallback,
        include_reflection,
        source_breakdown: summarize_counts(
            hits.iter().map(|item| source_bucket(item.source.as_str())),
        ),
        kind_breakdown: summarize_counts(hits.iter().map(|item| item.kind.as_str().trim())),
    }
}

fn truncate_for_trace(input: &str, max_chars: usize) -> String {
    let trimmed = input.trim();
    let mut chars = trimmed.chars();
    let preview: String = chars.by_ref().take(max_chars).collect();
    if chars.next().is_some() {
        format!("{}...", preview)
    } else {
        preview
    }
}

fn source_bucket(source: &str) -> &str {
    source.split(':').next().unwrap_or(source)
}

fn summarize_counts<'a>(labels: impl Iterator<Item = &'a str>) -> Vec<MemoryTraceCount> {
    let mut counts = BTreeMap::<String, usize>::new();
    for label in labels {
        let normalized = label.trim();
        if normalized.is_empty() {
            continue;
        }
        *counts.entry(normalized.to_string()).or_insert(0) += 1;
    }

    let mut pairs = counts
        .into_iter()
        .map(|(label, count)| MemoryTraceCount { label, count })
        .collect::<Vec<_>>();
    pairs.sort_by(|a, b| b.count.cmp(&a.count).then_with(|| a.label.cmp(&b.label)));
    pairs
}

#[cfg(test)]
mod tests {
    use super::{build_memory_retrieval_trace, DurableMemoryDiagnosticsItem};
    use crate::agents::RetrievedMemoryItem;
    use sentinel_db::core::models::database::{DurableMemoryProjectionState, DurableMemoryRecord};

    #[test]
    fn memory_retrieval_trace_summarizes_hits() {
        let hits = vec![
            RetrievedMemoryItem {
                id: "1".to_string(),
                text: "remember lexical".to_string(),
                kind: "fact".to_string(),
                scope: "project".to_string(),
                stability: "stable".to_string(),
                source: "memory_tool:canonical".to_string(),
                confidence: 0.9,
                importance: 3,
                created_at_ms: 1,
                score: 0.8,
            },
            RetrievedMemoryItem {
                id: "2".to_string(),
                text: "remember vector".to_string(),
                kind: "fact".to_string(),
                scope: "project".to_string(),
                stability: "stable".to_string(),
                source: "rag".to_string(),
                confidence: 0.8,
                importance: 4,
                created_at_ms: 2,
                score: 0.7,
            },
            RetrievedMemoryItem {
                id: "3".to_string(),
                text: "remember task".to_string(),
                kind: "task".to_string(),
                scope: "session".to_string(),
                stability: "tentative".to_string(),
                source: "memory_tool".to_string(),
                confidence: 0.7,
                importance: 2,
                created_at_ms: 3,
                score: 0.6,
            },
        ];

        let trace = build_memory_retrieval_trace(
            "   query with plenty of whitespace   ",
            5,
            &hits,
            true,
            false,
        );

        assert_eq!(trace.hit_count, 3);
        assert!(trace.used_canonical_fallback);
        assert_eq!(trace.source_breakdown[0].label, "memory_tool");
        assert_eq!(trace.source_breakdown[0].count, 2);
        assert_eq!(trace.kind_breakdown[0].label, "fact");
        assert_eq!(trace.kind_breakdown[0].count, 2);
        assert_eq!(trace.query_preview, "query with plenty of whitespace");
    }

    #[test]
    fn diagnostics_treats_lexical_memory_as_ready_without_vector_projection() {
        let item = DurableMemoryDiagnosticsItem::from_parts(
            memory_record("mem-1"),
            Some(DurableMemoryProjectionState {
                memory_id: "mem-1".to_string(),
                lexical_indexed: true,
                vector_indexed: false,
                skill_projected: true,
                last_error: Some("vector projection skipped or unavailable".to_string()),
                updated_at_ms: 2,
            }),
        );

        assert!(item.retrievable_projection_ready);
        assert!(!item.projection_issue);
    }

    #[test]
    fn diagnostics_flags_memory_without_any_retrievable_projection() {
        let item = DurableMemoryDiagnosticsItem::from_parts(
            memory_record("mem-2"),
            Some(DurableMemoryProjectionState {
                memory_id: "mem-2".to_string(),
                lexical_indexed: false,
                vector_indexed: false,
                skill_projected: true,
                last_error: None,
                updated_at_ms: 2,
            }),
        );

        assert!(!item.retrievable_projection_ready);
        assert!(item.projection_issue);
    }

    fn memory_record(id: &str) -> DurableMemoryRecord {
        DurableMemoryRecord {
            id: id.to_string(),
            title: Some("Preference".to_string()),
            text: "用户喜欢吃肉".to_string(),
            kind: "preference".to_string(),
            tier: "durable".to_string(),
            scope: "project".to_string(),
            stability: "stable".to_string(),
            source: "memory_tool".to_string(),
            confidence: 0.7,
            importance: 4,
            tags_json: "[]".to_string(),
            origin_execution_id: Some("memory_tool".to_string()),
            supersedes_memory_id: None,
            status: "active".to_string(),
            created_at_ms: 1,
            updated_at_ms: 2,
        }
    }
}
