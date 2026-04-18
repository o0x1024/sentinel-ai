//! Memory index with hybrid retrieval (keyword + vector via SQLite).

use anyhow::Result;
use chrono::Utc;
use std::collections::HashMap;
use std::sync::Arc;
use tauri::{AppHandle, Manager};

use crate::agents::context_engineering::checkpoint::{ContextMemoryItem, ContextRunState};
use sentinel_rag::{
    build_memory_document_id, build_memory_durable_metadata, canonicalize_memory_source,
    canonicalize_memory_stability, extract_memory_identifiers, infer_memory_kind,
    memory_kind_importance, normalize_memory_text, MemoryLexicalDocument, MemoryLexicalHit,
};

const MAX_MEMORY_ITEMS: usize = 200;
const RECENCY_HALF_LIFE_DAYS: f64 = 14.0;
const MEMORY_COLLECTION_NAME: &str = "agent_memory";
const RRF_K: f64 = 60.0;
const VECTOR_RRF_WEIGHT: f64 = 1.0;
const LEXICAL_RRF_WEIGHT: f64 = 0.9;
const KEYWORD_RRF_WEIGHT: f64 = 0.75;

#[derive(Debug, Clone)]
struct LocalKeywordHit {
    id: String,
    text: String,
    kind: String,
    scope: String,
    stability: String,
    source: String,
    confidence: f64,
    importance: u8,
    created_at_ms: i64,
    keyword_score: f64,
}

#[derive(Debug, Clone)]
struct MemoryCandidate {
    id: String,
    text: String,
    kind: String,
    scope: String,
    stability: String,
    source: String,
    confidence: f64,
    importance: u8,
    created_at_ms: i64,
    vector_rank: Option<usize>,
    vector_score: Option<f64>,
    lexical_rank: Option<usize>,
    lexical_bm25: Option<f64>,
    keyword_rank: Option<usize>,
    keyword_score: Option<f64>,
}

#[derive(Debug, Clone)]
pub struct MemoryQuery {
    pub execution_id: String,
    pub query: String,
    pub top_k: usize,
    pub include_reflection: bool,
}

#[derive(Debug, Clone)]
pub struct RetrievedMemoryItem {
    pub id: String,
    pub text: String,
    pub kind: String,
    pub scope: String,
    pub stability: String,
    pub source: String,
    pub confidence: f64,
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
        let inferred_kind = infer_memory_kind(None, None, &[], text);
        push_memory(
            state,
            text,
            inferred_kind.as_str(),
            memory_kind_importance(inferred_kind.as_str()),
        );
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
        .map(|t| (t.clone(), infer_memory_kind(None, None, &[], t)))
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
        .filter(|item| query.include_reflection || item.kind != "reflection")
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
            let durable_metadata = build_memory_durable_metadata(
                Some("session"),
                None,
                Some("run_state"),
                None,
                item.kind.as_str(),
                &[],
            );
            RetrievedMemoryItem {
                id: item.id.clone(),
                text: item.text.clone(),
                kind: item.kind.clone(),
                scope: durable_metadata.scope,
                stability: durable_metadata.stability,
                source: durable_metadata.source,
                confidence: durable_metadata.confidence,
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

    // 1) Snapshot local keyword candidates (avoids holding mutable borrow across async)
    let keyword_scores: HashMap<String, (f64, String, String, u8, i64)> = state
        .memory_items
        .iter()
        .filter(|item| query.include_reflection || item.kind != "reflection")
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
    let mut local_keyword_hits = state
        .memory_items
        .iter()
        .filter(|item| query.include_reflection || item.kind != "reflection")
        .filter_map(|item| {
            let keyword_score = keyword_score_raw(&query.query, &item.text);
            if keyword_score <= 0.0 {
                return None;
            }
            let durable_metadata = build_memory_durable_metadata(
                Some("session"),
                None,
                Some("run_state"),
                None,
                item.kind.as_str(),
                &[],
            );
            Some(LocalKeywordHit {
                id: item.id.clone(),
                text: item.text.clone(),
                kind: item.kind.clone(),
                scope: durable_metadata.scope,
                stability: durable_metadata.stability,
                source: durable_metadata.source,
                confidence: durable_metadata.confidence,
                importance: item.importance,
                created_at_ms: item.created_at_ms,
                keyword_score,
            })
        })
        .collect::<Vec<_>>();
    local_keyword_hits.sort_by(|a, b| {
        b.keyword_score
            .partial_cmp(&a.keyword_score)
            .unwrap_or(std::cmp::Ordering::Equal)
            .then_with(|| b.created_at_ms.cmp(&a.created_at_ms))
    });
    local_keyword_hits.truncate(query.top_k.max(1) * 2);

    // 2) Retrieve durable vector and lexical candidates in parallel
    let durable_top_k = query.top_k.max(1) * 2;
    let (vector_result, lexical_result) = tokio::join!(
        vector_retrieve(app_handle, &query.query, durable_top_k),
        lexical_retrieve(app_handle, &query.query, durable_top_k)
    );
    let vector_results = match vector_result {
        Ok(results) => results,
        Err(e) => {
            tracing::warn!(
                "Vector retrieval failed, continuing without vector candidates: {}",
                e
            );
            Vec::new()
        }
    };
    let lexical_results = match lexical_result {
        Ok(results) => results,
        Err(e) => {
            tracing::warn!(
                "Lexical retrieval failed, continuing without lexical candidates: {}",
                e
            );
            Vec::new()
        }
    };
    if vector_results.is_empty() && lexical_results.is_empty() && local_keyword_hits.is_empty() {
        return Vec::new();
    }

    // 3) Merge candidates by normalized text, then fuse by rank
    let mut candidates: HashMap<String, MemoryCandidate> = HashMap::new();

    for (index, (text, vec_score)) in vector_results.iter().enumerate() {
        let normalized_text = text.trim().to_string();
        if normalized_text.is_empty() {
            continue;
        }

        let (ks, id, kind, importance, created_at_ms) =
            if let Some((kw_score, item_id, item_kind, imp, cat)) =
                keyword_scores.get(&normalized_text)
            {
                (*kw_score, item_id.clone(), item_kind.clone(), *imp, *cat)
            } else {
                let ks = keyword_score_raw(&query.query, &normalized_text);
                (
                    ks,
                    build_memory_document_id("fact", normalized_text.as_str()),
                    "fact".to_string(),
                    3u8,
                    now_ms,
                )
            };
        let durable_metadata = build_memory_durable_metadata(
            Some("project"),
            None,
            Some("context_engineering"),
            None,
            kind.as_str(),
            &[],
        );

        let entry = candidates
            .entry(normalized_text.clone())
            .or_insert_with(|| MemoryCandidate {
                id,
                text: normalized_text,
                kind,
                scope: durable_metadata.scope,
                stability: durable_metadata.stability,
                source: durable_metadata.source,
                confidence: durable_metadata.confidence,
                importance,
                created_at_ms,
                vector_rank: None,
                vector_score: None,
                lexical_rank: None,
                lexical_bm25: None,
                keyword_rank: None,
                keyword_score: (ks > 0.0).then_some(ks),
            });
        entry.vector_rank = Some(index);
        entry.vector_score = Some(*vec_score);
        entry.keyword_score = entry.keyword_score.or((ks > 0.0).then_some(ks));
    }

    for (index, hit) in lexical_results.iter().enumerate() {
        let normalized_text = hit.body.trim().to_string();
        if normalized_text.is_empty() {
            continue;
        }

        let ks = keyword_score_raw(&query.query, &normalized_text);
        let entry = candidates
            .entry(normalized_text.clone())
            .or_insert_with(|| MemoryCandidate {
                id: hit.id.clone(),
                text: normalized_text,
                kind: hit.kind.clone(),
                scope: hit.scope.clone(),
                stability: hit.stability.clone(),
                source: hit.source.clone(),
                confidence: hit.confidence,
                importance: hit.importance,
                created_at_ms: hit.created_at_ms,
                vector_rank: None,
                vector_score: None,
                lexical_rank: None,
                lexical_bm25: None,
                keyword_rank: None,
                keyword_score: (ks > 0.0).then_some(ks),
            });
        entry.lexical_rank = Some(index);
        entry.lexical_bm25 = Some(hit.bm25_score);
        if entry.id.trim().is_empty() {
            entry.id = hit.id.clone();
        }
        if entry.kind.trim().is_empty() {
            entry.kind = hit.kind.clone();
        }
        if entry.scope.trim().is_empty() {
            entry.scope = hit.scope.clone();
        }
        if entry.stability.trim().is_empty() {
            entry.stability = hit.stability.clone();
        }
        if entry.source.trim().is_empty() {
            entry.source = hit.source.clone();
        }
        entry.confidence = entry.confidence.max(hit.confidence);
        entry.importance = entry.importance.max(hit.importance);
        entry.created_at_ms = entry.created_at_ms.min(hit.created_at_ms);
    }

    for (index, hit) in local_keyword_hits.iter().enumerate() {
        let normalized_text = hit.text.trim().to_string();
        if normalized_text.is_empty() {
            continue;
        }

        let entry = candidates
            .entry(normalized_text.clone())
            .or_insert_with(|| MemoryCandidate {
                id: hit.id.clone(),
                text: normalized_text,
                kind: hit.kind.clone(),
                scope: hit.scope.clone(),
                stability: hit.stability.clone(),
                source: hit.source.clone(),
                confidence: hit.confidence,
                importance: hit.importance,
                created_at_ms: hit.created_at_ms,
                vector_rank: None,
                vector_score: None,
                lexical_rank: None,
                lexical_bm25: None,
                keyword_rank: None,
                keyword_score: Some(hit.keyword_score),
            });
        entry.keyword_rank = Some(index);
        entry.keyword_score = Some(hit.keyword_score);
        entry.scope = hit.scope.clone();
        entry.stability = hit.stability.clone();
        entry.source = hit.source.clone();
        entry.confidence = entry.confidence.max(hit.confidence);
        entry.importance = entry.importance.max(hit.importance);
        entry.created_at_ms = entry.created_at_ms.min(hit.created_at_ms);
    }

    let mut scored: Vec<RetrievedMemoryItem> = candidates
        .into_values()
        .filter_map(|candidate| {
            let score = fused_candidate_score(&candidate, now_ms);
            if score <= 0.0 {
                return None;
            }
            Some(RetrievedMemoryItem {
                id: candidate.id,
                text: candidate.text,
                kind: candidate.kind,
                scope: candidate.scope,
                stability: candidate.stability,
                source: candidate.source,
                confidence: candidate.confidence,
                importance: candidate.importance,
                created_at_ms: candidate.created_at_ms,
                score,
            })
        })
        .collect();

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

async fn lexical_retrieve(
    app_handle: &AppHandle,
    query_text: &str,
    top_k: usize,
) -> Result<Vec<MemoryLexicalHit>> {
    let db = app_handle
        .try_state::<Arc<sentinel_db::DatabaseService>>()
        .ok_or_else(|| anyhow::anyhow!("DatabaseService not available"))?;

    let rag_service = crate::commands::rag_commands::get_or_init_rag_service(db.inner().clone())
        .await
        .map_err(|e| anyhow::anyhow!("RAG service init failed: {}", e))?;

    rag_service
        .search_memory_lexical(MEMORY_COLLECTION_NAME, query_text, top_k)
        .await
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

        let inferred_kind = infer_memory_kind(Some(kind.as_str()), None, &[], trimmed);
        let durable_metadata = build_memory_durable_metadata(
            Some("project"),
            None,
            Some("context_engineering"),
            None,
            inferred_kind.as_str(),
            &[],
        );
        let title = format!("[{}] {}", inferred_kind, truncate_str(trimmed, 80));
        let mut metadata = HashMap::new();
        metadata.insert("type".to_string(), "agent_memory".to_string());
        metadata.insert("kind".to_string(), inferred_kind.clone());
        metadata.insert("scope".to_string(), durable_metadata.scope.clone());
        metadata.insert("stability".to_string(), durable_metadata.stability.clone());
        metadata.insert("source".to_string(), durable_metadata.source.clone());
        metadata.insert(
            "confidence".to_string(),
            format!("{:.2}", durable_metadata.confidence),
        );
        metadata.insert(
            "created_at".to_string(),
            Utc::now().timestamp_millis().to_string(),
        );

        if let Err(e) = rag_service
            .ingest_text(&title, trimmed, Some(&collection_id), Some(metadata))
            .await
        {
            tracing::warn!("Failed to ingest memory item to vector store: {}", e);
            continue;
        }

        let now_ms = Utc::now().timestamp_millis();
        let candidate_scope = durable_metadata.scope.clone();
        let candidate_stability = durable_metadata.stability.clone();
        let candidate_source = durable_metadata.source.clone();
        let candidate_confidence = durable_metadata.confidence;
        let lexical_doc = MemoryLexicalDocument {
            id: build_memory_document_id(inferred_kind.as_str(), trimmed),
            collection_name: MEMORY_COLLECTION_NAME.to_string(),
            title: Some(title),
            body: trimmed.to_string(),
            normalized_text: normalize_memory_text(trimmed),
            identifiers: extract_memory_identifiers(trimmed),
            tags: inferred_kind.clone(),
            kind: inferred_kind.clone(),
            scope: durable_metadata.scope,
            stability: durable_metadata.stability,
            source: durable_metadata.source,
            confidence: candidate_confidence,
            importance: memory_kind_importance(inferred_kind.as_str()),
            created_at_ms: now_ms,
            updated_at_ms: now_ms,
        };

        if let Err(e) = rag_service
            .upsert_memory_lexical_document(lexical_doc)
            .await
        {
            tracing::warn!("Failed to index memory item in lexical store: {}", e);
        }

        if let Err(e) = crate::skills::candidates::upsert_skill_candidate_from_memory(
            db.inner().as_ref(),
            &crate::skills::candidates::SkillCandidateMemoryInput {
                memory_id: Some(build_memory_document_id(inferred_kind.as_str(), trimmed)),
                text: trimmed.to_string(),
                kind: inferred_kind.clone(),
                scope: candidate_scope,
                source: candidate_source,
                stability: candidate_stability,
                confidence: candidate_confidence,
            },
        ) {
            tracing::warn!(
                "Failed to capture skill candidate from durable memory: {}",
                e
            );
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

pub fn keyword_score_value(query_text: &str, item_text: &str) -> f64 {
    keyword_score_raw(query_text, item_text)
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

fn lexical_score_to_signal(bm25_score: f64) -> f64 {
    let adjusted = if bm25_score.is_finite() {
        bm25_score.max(0.0)
    } else {
        5.0
    };
    (1.0 / (1.0 + adjusted)).clamp(0.05, 1.0)
}

fn reciprocal_rank(rank: Option<usize>, weight: f64) -> f64 {
    rank.map(|value| weight / (RRF_K + value as f64 + 1.0))
        .unwrap_or(0.0)
}

fn stability_signal(stability: &str) -> f64 {
    match canonicalize_memory_stability(stability).as_str() {
        "stable" => 0.05,
        "tentative" => -0.02,
        _ => 0.0,
    }
}

fn source_signal(source: &str) -> f64 {
    match canonicalize_memory_source(source).as_str() {
        "memory_tool" => 0.04,
        "context_engineering" => 0.02,
        "legacy" => 0.0,
        _ => 0.0,
    }
}

fn fused_candidate_score(candidate: &MemoryCandidate, now_ms: i64) -> f64 {
    let vector_rrf = reciprocal_rank(candidate.vector_rank, VECTOR_RRF_WEIGHT);
    let lexical_rrf = reciprocal_rank(candidate.lexical_rank, LEXICAL_RRF_WEIGHT);
    let keyword_rrf = reciprocal_rank(candidate.keyword_rank, KEYWORD_RRF_WEIGHT);

    let vector_signal = candidate.vector_score.unwrap_or(0.0) * 0.35;
    let lexical_signal = candidate
        .lexical_bm25
        .map(lexical_score_to_signal)
        .unwrap_or(0.0)
        * 0.18;
    let keyword_signal = candidate.keyword_score.unwrap_or(0.0) * 0.12;
    let importance_signal = (candidate.importance as f64 / 5.0).clamp(0.2, 1.0) * 0.06;
    let recency_signal = recency_score(candidate.created_at_ms, now_ms) * 0.04;
    let confidence_signal = candidate.confidence.clamp(0.0, 1.0) * 0.08;
    let stability_signal = stability_signal(candidate.stability.as_str());
    let source_signal = source_signal(candidate.source.as_str());

    vector_rrf
        + lexical_rrf
        + keyword_rrf
        + vector_signal
        + lexical_signal
        + keyword_signal
        + importance_signal
        + recency_signal
        + confidence_signal
        + stability_signal
        + source_signal
}
