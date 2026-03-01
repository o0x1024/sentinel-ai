//! Memory index with hybrid retrieval (keyword + vector via SQLite).

use anyhow::Result;
use chrono::Utc;
use std::collections::HashMap;
use std::sync::Arc;
use tauri::{AppHandle, Manager};

use crate::agents::context_engineering::checkpoint::{ContextMemoryItem, ContextRunState};

const MAX_MEMORY_ITEMS: usize = 200;
const VECTOR_WEIGHT: f64 = 0.55;
const KEYWORD_WEIGHT: f64 = 0.25;
const IMPORTANCE_WEIGHT: f64 = 0.12;
const RECENCY_WEIGHT: f64 = 0.08;
const RECENCY_HALF_LIFE_DAYS: f64 = 14.0;

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

// ---------------------------------------------------------------------------
// Ingestion
// ---------------------------------------------------------------------------

pub fn ingest_memory_items(
    state: &mut ContextRunState,
    facts: &[String],
    decisions: &[String],
    todos: &[String],
) {
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
    if state.memory_items.len() > MAX_MEMORY_ITEMS {
        let keep_from = state.memory_items.len() - MAX_MEMORY_ITEMS;
        state.memory_items = state.memory_items.split_off(keep_from);
    }
}

/// Ingest memory items and persist them to SQLite for cross-session retrieval.
pub async fn ingest_memory_items_persistent(
    app_handle: &AppHandle,
    state: &mut ContextRunState,
    facts: &[String],
    decisions: &[String],
    todos: &[String],
) {
    ingest_memory_items(state, facts, decisions, todos);

    let items_to_persist: Vec<(String, String)> = facts
        .iter()
        .map(|t| (t.clone(), "fact".to_string()))
        .chain(
            decisions
                .iter()
                .map(|t| (t.clone(), "decision".to_string())),
        )
        .chain(todos.iter().map(|t| (t.clone(), "todo".to_string())))
        .filter(|(t, _)| !t.trim().is_empty())
        .collect();

    if items_to_persist.is_empty() {
        return;
    }

    if let Err(e) = persist_to_vector_store(app_handle, &items_to_persist).await {
        tracing::warn!("Failed to persist memory items to vector store: {}", e);
    }
}

// ---------------------------------------------------------------------------
// Retrieval — keyword only (sync, backward-compatible)
// ---------------------------------------------------------------------------

pub fn retrieve_memory_items(
    state: &mut ContextRunState,
    query: &MemoryQuery,
) -> Vec<RetrievedMemoryItem> {
    let now_ms = Utc::now().timestamp_millis();
    let mut items = state
        .memory_items
        .iter_mut()
        .map(|item| {
            let score = keyword_score(query, item, now_ms);
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

// ---------------------------------------------------------------------------
// Retrieval — hybrid (keyword + vector via SQLite)
// ---------------------------------------------------------------------------

pub async fn retrieve_memory_items_hybrid(
    app_handle: &AppHandle,
    state: &mut ContextRunState,
    query: &MemoryQuery,
) -> Vec<RetrievedMemoryItem> {
    let now_ms = Utc::now().timestamp_millis();

    // 1) Snapshot keyword scores (avoids holding mutable borrow across async)
    let keyword_scores: HashMap<String, (f64, String, String, u8, i64)> = state
        .memory_items
        .iter()
        .map(|item| {
            let ks = keyword_score_raw(&query.query, &item.text);
            (
                item.text.clone(),
                (
                    ks,
                    item.id.clone(),
                    item.kind.clone(),
                    item.importance,
                    item.created_at_ms,
                ),
            )
        })
        .collect();

    // 2) Get vector scores from SQLite vector store
    let vector_results = match vector_retrieve(app_handle, &query.query, query.top_k * 2).await {
        Ok(results) => results,
        Err(e) => {
            tracing::warn!(
                "Vector retrieval failed, falling back to keyword-only: {}",
                e
            );
            return retrieve_memory_items(state, query);
        }
    };

    // 3) Merge: combine vector results with keyword scores
    let mut scored: Vec<RetrievedMemoryItem> = Vec::new();
    let mut seen_texts: std::collections::HashSet<String> = std::collections::HashSet::new();

    for (text, vec_score) in &vector_results {
        let normalized_text = text.trim().to_string();
        if normalized_text.is_empty() || !seen_texts.insert(normalized_text.clone()) {
            continue;
        }

        let (ks, importance, kind, id, created_at_ms) =
            if let Some((kw_score, item_id, item_kind, imp, cat)) =
                keyword_scores.get(&normalized_text)
            {
                (*kw_score, *imp, item_kind.clone(), item_id.clone(), *cat)
            } else {
                let ks = keyword_score_raw(&query.query, &normalized_text);
                (
                    ks,
                    3u8,
                    "fact".to_string(),
                    uuid::Uuid::new_v4().to_string(),
                    now_ms,
                )
            };

        let recency = recency_score(created_at_ms, now_ms);
        let imp = (importance as f64 / 5.0).clamp(0.2, 1.0);
        let hybrid = (vec_score * VECTOR_WEIGHT)
            + (ks * KEYWORD_WEIGHT)
            + (imp * IMPORTANCE_WEIGHT)
            + (recency * RECENCY_WEIGHT);

        if hybrid > 0.05 {
            scored.push(RetrievedMemoryItem {
                id,
                text: normalized_text,
                kind,
                importance,
                created_at_ms,
                score: hybrid,
            });
        }
    }

    // Add keyword-only hits not covered by vector results
    for (text, (ks, item_id, item_kind, imp, cat)) in &keyword_scores {
        if *ks > 0.0 && !seen_texts.contains(text) {
            seen_texts.insert(text.clone());
            let recency = recency_score(*cat, now_ms);
            let importance_norm = (*imp as f64 / 5.0).clamp(0.2, 1.0);
            let hybrid = (ks * (VECTOR_WEIGHT + KEYWORD_WEIGHT))
                + (importance_norm * IMPORTANCE_WEIGHT)
                + (recency * RECENCY_WEIGHT);

            scored.push(RetrievedMemoryItem {
                id: item_id.clone(),
                text: text.clone(),
                kind: item_kind.clone(),
                importance: *imp,
                created_at_ms: *cat,
                score: hybrid,
            });
        }
    }

    scored.sort_by(|a, b| {
        b.score
            .partial_cmp(&a.score)
            .unwrap_or(std::cmp::Ordering::Equal)
    });
    scored.truncate(query.top_k.max(1));

    // Update last_used_at for retrieved items
    let retrieved_texts: std::collections::HashSet<String> =
        scored.iter().map(|r| r.text.clone()).collect();
    for item in state.memory_items.iter_mut() {
        if retrieved_texts.contains(&item.text) {
            item.last_used_at_ms = now_ms;
        }
    }

    scored
}

// ---------------------------------------------------------------------------
// Vector store integration (SQLite via RAG service)
// ---------------------------------------------------------------------------

async fn vector_retrieve(
    app_handle: &AppHandle,
    query_text: &str,
    top_k: usize,
) -> Result<Vec<(String, f64)>> {
    let db = app_handle
        .try_state::<Arc<sentinel_db::DatabaseService>>()
        .ok_or_else(|| anyhow::anyhow!("DatabaseService not available"))?;

    let rag_service = crate::commands::rag_commands::get_or_init_rag_service(db.inner().clone())
        .await
        .map_err(|e| anyhow::anyhow!("RAG service init failed: {}", e))?;

    let collection_id =
        crate::commands::rag_commands::ensure_memory_collection_exists(db.inner().clone())
            .await
            .map_err(|e| anyhow::anyhow!("Memory collection error: {}", e))?;

    let request = sentinel_rag::RagQueryRequest {
        query: query_text.to_string(),
        collection_id: Some(collection_id),
        top_k: Some(top_k),
        use_embedding: Some(true),
        // Use configured RAG similarity threshold (from settings) instead of hard-coded value.
        similarity_threshold: None,
        use_mmr: None,
        mmr_lambda: None,
        filters: None,
        reranking_enabled: None,
    };

    let response = rag_service.query(request).await?;

    let results: Vec<(String, f64)> = response
        .results
        .into_iter()
        .map(|r| (r.chunk.content, r.score))
        .collect();

    Ok(results)
}

async fn persist_to_vector_store(app_handle: &AppHandle, items: &[(String, String)]) -> Result<()> {
    let db = app_handle
        .try_state::<Arc<sentinel_db::DatabaseService>>()
        .ok_or_else(|| anyhow::anyhow!("DatabaseService not available"))?;

    let rag_service = crate::commands::rag_commands::get_or_init_rag_service(db.inner().clone())
        .await
        .map_err(|e| anyhow::anyhow!("RAG service init failed: {}", e))?;

    let collection_id =
        crate::commands::rag_commands::ensure_memory_collection_exists(db.inner().clone())
            .await
            .map_err(|e| anyhow::anyhow!("Memory collection error: {}", e))?;

    for (text, kind) in items {
        let trimmed = text.trim();
        if trimmed.is_empty() {
            continue;
        }

        // Deduplicate: check if similar content already exists
        let check_request = sentinel_rag::RagQueryRequest {
            query: trimmed.to_string(),
            collection_id: Some(collection_id.clone()),
            top_k: Some(1),
            use_embedding: Some(true),
            similarity_threshold: Some(0.92),
            use_mmr: None,
            mmr_lambda: None,
            filters: None,
            reranking_enabled: None,
        };

        let existing = rag_service.query(check_request).await;
        if let Ok(resp) = existing {
            if !resp.results.is_empty() && resp.results[0].score > 0.92 {
                tracing::debug!(
                    "Skipping duplicate memory item (score={:.3})",
                    resp.results[0].score
                );
                continue;
            }
        }

        let title = format!("[{}] {}", kind, truncate_str(trimmed, 80));
        let mut metadata = HashMap::new();
        metadata.insert("type".to_string(), "agent_memory".to_string());
        metadata.insert("kind".to_string(), kind.clone());
        metadata.insert(
            "created_at".to_string(),
            Utc::now().timestamp_millis().to_string(),
        );

        if let Err(e) = rag_service
            .ingest_text(&title, trimmed, Some(&collection_id), Some(metadata))
            .await
        {
            tracing::warn!("Failed to ingest memory item to vector store: {}", e);
        }
    }

    Ok(())
}

// ---------------------------------------------------------------------------
// Scoring helpers
// ---------------------------------------------------------------------------

fn keyword_score(query: &MemoryQuery, item: &ContextMemoryItem, now_ms: i64) -> f64 {
    let ks = keyword_score_raw(&query.query, &item.text);
    if ks <= 0.0 {
        return 0.0;
    }
    let recency = recency_score(item.created_at_ms, now_ms);
    let importance = (item.importance as f64 / 5.0).clamp(0.2, 1.0);
    (ks * 0.6) + (importance * 0.25) + (recency * 0.15)
}

fn keyword_score_raw(query_text: &str, item_text: &str) -> f64 {
    let terms = tokenize(query_text);
    if terms.is_empty() {
        return 0.0;
    }
    let text = item_text.to_lowercase();
    let text_terms = tokenize(&text);
    let mut hit = 0f64;
    for term in &terms {
        if text.contains(term) || text_terms.iter().any(|c| term_matches(term, c)) {
            hit += 1.0;
        }
    }
    if hit <= 0.0 {
        return 0.0;
    }
    (hit / terms.len() as f64).clamp(0.1, 1.0)
}

fn recency_score(created_at_ms: i64, now_ms: i64) -> f64 {
    let days = ((now_ms - created_at_ms).max(0) as f64) / 86_400_000f64;
    (RECENCY_HALF_LIFE_DAYS / (RECENCY_HALF_LIFE_DAYS + days)).clamp(0.05, 1.0)
}

// ---------------------------------------------------------------------------
// Eviction — importance × usage frequency × time decay
// ---------------------------------------------------------------------------

/// Smart eviction: remove low-value items when approaching capacity.
/// Called during ingestion when items exceed MAX_MEMORY_ITEMS.
pub fn evict_low_value_items(state: &mut ContextRunState) {
    if state.memory_items.len() <= MAX_MEMORY_ITEMS {
        return;
    }
    let now_ms = Utc::now().timestamp_millis();
    let mut scored: Vec<(usize, f64)> = state
        .memory_items
        .iter()
        .enumerate()
        .map(|(idx, item)| {
            let imp = (item.importance as f64 / 5.0).clamp(0.1, 1.0);
            let recency = recency_score(item.created_at_ms, now_ms);
            let usage_recency = recency_score(item.last_used_at_ms, now_ms);
            let value = (imp * 0.4) + (recency * 0.3) + (usage_recency * 0.3);
            (idx, value)
        })
        .collect();

    scored.sort_by(|a, b| a.1.partial_cmp(&b.1).unwrap_or(std::cmp::Ordering::Equal));
    let to_remove = state.memory_items.len() - MAX_MEMORY_ITEMS;
    let remove_indices: std::collections::HashSet<usize> =
        scored.iter().take(to_remove).map(|(idx, _)| *idx).collect();

    let mut kept = Vec::with_capacity(MAX_MEMORY_ITEMS);
    for (idx, item) in state.memory_items.drain(..).enumerate() {
        if !remove_indices.contains(&idx) {
            kept.push(item);
        }
    }
    state.memory_items = kept;
}

// ---------------------------------------------------------------------------
// Internal helpers (preserved from original)
// ---------------------------------------------------------------------------

fn push_memory(state: &mut ContextRunState, text: &str, kind: &str, importance: u8) {
    let trimmed = text.trim();
    if trimmed.is_empty() {
        return;
    }

    if state
        .memory_items
        .iter()
        .any(|item| item.text == trimmed && item.kind == kind)
    {
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

fn is_cjk(c: char) -> bool {
    matches!(c as u32,
        0x4E00..=0x9FFF
        | 0x3400..=0x4DBF
        | 0x20000..=0x2A6DF
        | 0xF900..=0xFAFF
        | 0x2F800..=0x2FA1F
    )
}

fn tokenize(text: &str) -> Vec<String> {
    let mut tokens = Vec::new();
    let mut current = String::new();

    for c in text.to_lowercase().chars() {
        if is_cjk(c) {
            if !current.is_empty() {
                let normalized = normalize_term(&current);
                if normalized.len() > 1 {
                    tokens.push(normalized);
                }
                current.clear();
            }
            tokens.push(c.to_string());
        } else if c.is_alphanumeric() || c == '_' || c == '-' || c == '.' {
            current.push(c);
        } else if !current.is_empty() {
            let normalized = normalize_term(&current);
            if normalized.len() > 1 {
                tokens.push(normalized);
            }
            current.clear();
        }
    }
    if !current.is_empty() {
        let normalized = normalize_term(&current);
        if normalized.len() > 1 {
            tokens.push(normalized);
        }
    }
    tokens
}

fn normalize_term(term: &str) -> String {
    let t = term.to_string();
    if t.len() < 5 {
        return t;
    }
    for suffix in [
        "tion", "sion", "ment", "ness", "ures", "ions", "ing", "ure", "ally", "ely",
    ] {
        if t.len() >= suffix.len() + 4 && t.ends_with(suffix) {
            return t[..t.len() - suffix.len()].to_string();
        }
    }
    if t.len() >= 6 && t.ends_with("ed") && !t.ends_with("eed") {
        return t[..t.len() - 2].to_string();
    }
    if t.len() >= 6 && t.ends_with("ly") {
        return t[..t.len() - 2].to_string();
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

fn truncate_str(s: &str, max_len: usize) -> &str {
    if s.len() <= max_len {
        return s;
    }
    match s.char_indices().nth(max_len) {
        Some((idx, _)) => &s[..idx],
        None => s,
    }
}
