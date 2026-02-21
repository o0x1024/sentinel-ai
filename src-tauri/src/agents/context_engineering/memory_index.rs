//! Lightweight memory index abstraction backed by run-state storage.

use chrono::Utc;

use crate::agents::context_engineering::checkpoint::{ContextMemoryItem, ContextRunState};

#[derive(Debug, Clone)]
pub struct MemoryQuery {
    pub execution_id: String,
    pub query: String,
    pub top_k: usize,
}

#[derive(Debug, Clone)]
pub struct RetrievedMemoryItem {
    pub id: String,
    pub text: String,
    pub kind: String,
    pub importance: u8,
    pub created_at_ms: i64,
    pub score: f64,
}

pub fn ingest_memory_items(state: &mut ContextRunState, facts: &[String], decisions: &[String], todos: &[String]) {
    for text in facts {
        push_memory(state, text, "fact", 3);
    }
    for text in decisions {
        push_memory(state, text, "decision", 4);
    }
    for text in todos {
        push_memory(state, text, "todo", 3);
    }
    state.memory_items.sort_by_key(|item| item.created_at_ms);
    if state.memory_items.len() > 200 {
        let keep_from = state.memory_items.len() - 200;
        state.memory_items = state.memory_items.split_off(keep_from);
    }
}

pub fn retrieve_memory_items(state: &mut ContextRunState, query: &MemoryQuery) -> Vec<RetrievedMemoryItem> {
    let now_ms = Utc::now().timestamp_millis();
    let mut items = state
        .memory_items
        .iter_mut()
        .map(|item| {
            let score = score_item(query, item, now_ms);
            (item, score)
        })
        .filter(|(_, score)| *score > 0.0)
        .collect::<Vec<_>>();

    items.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));
    let keep = query.top_k.max(1);
    items
        .into_iter()
        .take(keep)
        .map(|(item, score)| {
            item.last_used_at_ms = now_ms;
            RetrievedMemoryItem {
                id: item.id.clone(),
                text: item.text.clone(),
                kind: item.kind.clone(),
                importance: item.importance,
                created_at_ms: item.created_at_ms,
                score,
            }
        })
        .collect()
}

fn score_item(query: &MemoryQuery, item: &ContextMemoryItem, now_ms: i64) -> f64 {
    let terms = tokenize(&query.query);
    if terms.is_empty() {
        return 0.0;
    }
    let text = item.text.to_lowercase();
    let text_terms = tokenize(&text);
    let mut hit = 0f64;
    for term in &terms {
        if text.contains(term) || text_terms.iter().any(|candidate| term_matches(term, candidate))
        {
            hit += 1.0;
        }
    }
    if hit <= 0.0 {
        return 0.0;
    }

    let recency_days = ((now_ms - item.created_at_ms).max(0) as f64) / 86_400_000f64;
    let recency = (14f64 / (14f64 + recency_days)).clamp(0.05, 1.0);
    let importance = (item.importance as f64 / 5.0).clamp(0.2, 1.0);
    let coverage = (hit / terms.len() as f64).clamp(0.1, 1.0);
    (coverage * 0.6) + (importance * 0.25) + (recency * 0.15)
}

fn push_memory(state: &mut ContextRunState, text: &str, kind: &str, importance: u8) {
    let trimmed = text.trim();
    if trimmed.is_empty() {
        return;
    }

    if state.memory_items.iter().any(|item| item.text == trimmed && item.kind == kind) {
        return;
    }
    let now_ms = Utc::now().timestamp_millis();
    state.memory_items.push(ContextMemoryItem {
        id: uuid::Uuid::new_v4().to_string(),
        text: trimmed.to_string(),
        kind: kind.to_string(),
        importance,
        created_at_ms: now_ms,
        last_used_at_ms: now_ms,
    });
}

fn tokenize(text: &str) -> Vec<String> {
    text.split(|c: char| !(c.is_alphanumeric() || c == '_' || c == '-' || c == '.'))
        .map(|s| normalize_term(&s.trim().to_lowercase()))
        .filter(|s| s.len() > 1)
        .collect()
}

fn normalize_term(term: &str) -> String {
    let mut t = term.to_string();
    for suffix in ["ing", "ed", "ion", "ions", "ure", "ures", "ly", "s"] {
        if t.len() > suffix.len() + 2 && t.ends_with(suffix) {
            t.truncate(t.len() - suffix.len());
            break;
        }
    }
    t
}

fn term_matches(query: &str, candidate: &str) -> bool {
    if query == candidate {
        return true;
    }
    let q = normalize_term(query);
    let c = normalize_term(candidate);
    if q == c {
        return true;
    }
    let min_len = 4usize;
    q.len() >= min_len && c.len() >= min_len && (q.starts_with(&c) || c.starts_with(&q))
}
