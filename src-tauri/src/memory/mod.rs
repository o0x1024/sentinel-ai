pub mod diagnostics;
pub mod management;

use anyhow::{anyhow, Result};
use std::collections::HashMap;
use std::sync::Arc;
use tauri::{AppHandle, Manager};

use crate::agents::context_engineering::{
    retrieve_memory_unified, ContextRunState, MemoryQuery, RetrievedMemoryItem,
};
use crate::commands::rag_commands::{ensure_memory_collection_exists, get_or_init_rag_service};
use crate::skills::candidates::{upsert_skill_candidate_from_memory, SkillCandidateMemoryInput};
pub use diagnostics::{
    build_memory_retrieval_trace, DurableMemoryDiagnosticsItem, MemoryRetrievalTrace,
};
pub use management::{
    delete_durable_memory, filter_hits_for_auto_inject, memory_auto_inject_enabled,
    preview_memory_retrieval, set_durable_memory_auto_inject, update_durable_memory,
    DurableMemoryAutoInjectResult, DurableMemoryDeleteResult, DurableMemoryUpdateResult,
    NO_AUTO_INJECT_TAG,
};
use sentinel_db::core::models::database::{DurableMemoryProjectionState, DurableMemoryRecord};
use sentinel_tools::buildin_tools::memory::{
    MemoryManagerProjectionState, MemoryManagerResultItem, MemoryManagerRetrievalTrace,
    MemoryManagerRetrieveResult, MemoryManagerStoreResult, MemoryManagerTraceCount,
};

const MEMORY_TOOL_EXECUTION_ID: &str = "memory_tool";

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct MemoryRetrieveOutcome {
    pub hits: Vec<RetrievedMemoryItem>,
    pub trace: MemoryRetrievalTrace,
}

pub struct MemoryStoreOutcome {
    pub memory_id: String,
    pub title: Option<String>,
    pub kind: String,
    pub scope: String,
    pub stability: String,
    pub source: String,
    pub confidence: f64,
    pub created_at_ms: i64,
    pub projection: DurableMemoryProjectionState,
}

pub async fn store_memory(
    app_handle: &AppHandle,
    content: String,
    title: Option<String>,
    tags: Vec<String>,
) -> Result<MemoryStoreOutcome> {
    let db_service = app_handle
        .try_state::<Arc<sentinel_db::DatabaseService>>()
        .ok_or_else(|| anyhow!("DatabaseService not available"))?;
    let db = db_service.inner().clone();

    let inferred_kind = sentinel_rag::infer_memory_kind(None, title.as_deref(), &tags, &content);
    let durable_metadata = sentinel_rag::build_memory_durable_metadata(
        None,
        None,
        Some("memory_tool"),
        None,
        inferred_kind.as_str(),
        &tags,
    );
    let mut metadata = HashMap::new();
    if !tags.is_empty() {
        metadata.insert("tags".to_string(), tags.join(","));
    }
    metadata.insert("type".to_string(), "agent_memory".to_string());
    metadata.insert("kind".to_string(), inferred_kind.clone());
    metadata.insert("scope".to_string(), durable_metadata.scope.clone());
    metadata.insert("stability".to_string(), durable_metadata.stability.clone());
    metadata.insert("source".to_string(), durable_metadata.source.clone());
    metadata.insert(
        "confidence".to_string(),
        format!("{:.2}", durable_metadata.confidence),
    );

    let final_title = if let Some(value) = title {
        if value.trim().is_empty() {
            format!(
                "[{}] {}",
                inferred_kind,
                content.chars().take(30).collect::<String>()
            )
        } else {
            value
        }
    } else {
        let snippet = content.chars().take(30).collect::<String>();
        format!("[{}] {}...", inferred_kind, snippet.trim())
    };

    let candidate_scope = durable_metadata.scope.clone();
    let candidate_stability = durable_metadata.stability.clone();
    let candidate_source = durable_metadata.source.clone();
    let candidate_confidence = durable_metadata.confidence;
    let now_ms = chrono::Utc::now().timestamp_millis();
    let memory_id =
        sentinel_rag::build_memory_document_id(inferred_kind.as_str(), content.as_str());
    metadata.insert("memory_id".to_string(), memory_id.clone());

    db.upsert_durable_memory_record_internal(&DurableMemoryRecord {
        id: memory_id.clone(),
        title: Some(final_title.clone()),
        text: content.clone(),
        kind: inferred_kind.clone(),
        tier: "durable".to_string(),
        scope: candidate_scope.clone(),
        stability: candidate_stability.clone(),
        source: candidate_source.clone(),
        confidence: candidate_confidence,
        importance: i32::from(sentinel_rag::memory_kind_importance(inferred_kind.as_str())),
        tags_json: serde_json::to_string(&tags).unwrap_or_else(|_| "[]".to_string()),
        origin_execution_id: Some(MEMORY_TOOL_EXECUTION_ID.to_string()),
        supersedes_memory_id: None,
        status: "active".to_string(),
        created_at_ms: now_ms,
        updated_at_ms: now_ms,
    })
    .await?;

    let mut projection_state = DurableMemoryProjectionState {
        memory_id: memory_id.clone(),
        lexical_indexed: false,
        vector_indexed: false,
        skill_projected: false,
        last_error: None,
        updated_at_ms: now_ms,
    };
    db.upsert_durable_memory_projection_state_internal(&projection_state)
        .await?;

    let mut errors = Vec::new();

    match get_or_init_rag_service(db.clone()).await {
        Ok(service) => {
            match ensure_memory_collection_exists(db.clone()).await {
                Ok(collection_id) => {
                    if let Err(e) = service
                        .ingest_text(&final_title, &content, Some(&collection_id), Some(metadata))
                        .await
                    {
                        errors.push(format!("vector projection failed: {}", e));
                    } else {
                        projection_state.vector_indexed = true;
                    }
                }
                Err(e) => {
                    errors.push(format!("memory collection unavailable: {}", e));
                }
            }

            if let Err(e) = service
                .upsert_memory_lexical_document(sentinel_rag::MemoryLexicalDocument {
                    id: memory_id.clone(),
                    collection_name: "agent_memory".to_string(),
                    title: Some(final_title.clone()),
                    body: content.clone(),
                    normalized_text: sentinel_rag::normalize_memory_text(content.as_str()),
                    identifiers: sentinel_rag::extract_memory_identifiers(content.as_str()),
                    tags: tags.join(","),
                    kind: inferred_kind.clone(),
                    scope: durable_metadata.scope,
                    stability: durable_metadata.stability,
                    source: durable_metadata.source,
                    confidence: candidate_confidence,
                    importance: sentinel_rag::memory_kind_importance(inferred_kind.as_str()),
                    created_at_ms: now_ms,
                    updated_at_ms: now_ms,
                })
                .await
            {
                errors.push(format!("lexical projection failed: {}", e));
            } else {
                projection_state.lexical_indexed = true;
            }
        }
        Err(e) => errors.push(format!("rag service unavailable: {}", e)),
    }

    if let Err(e) = upsert_skill_candidate_from_memory(
        db.as_ref(),
        &SkillCandidateMemoryInput {
            memory_id: Some(memory_id.clone()),
            text: content,
            kind: inferred_kind.clone(),
            scope: candidate_scope.clone(),
            source: candidate_source.clone(),
            stability: candidate_stability.clone(),
            confidence: candidate_confidence,
        },
    ) {
        errors.push(format!("skill projection failed: {}", e));
    } else {
        projection_state.skill_projected = true;
    }

    projection_state.last_error = (!errors.is_empty()).then(|| errors.join(" | "));
    projection_state.updated_at_ms = chrono::Utc::now().timestamp_millis();
    db.upsert_durable_memory_projection_state_internal(&projection_state)
        .await?;

    if !projection_state.vector_indexed && !projection_state.lexical_indexed {
        return Err(anyhow!(
            "Memory persisted to canonical store, but retrievable projections failed: {}",
            projection_state
                .last_error
                .unwrap_or_else(|| "unknown projection error".to_string())
        ));
    }

    Ok(MemoryStoreOutcome {
        memory_id,
        title: Some(final_title),
        kind: inferred_kind,
        scope: candidate_scope,
        stability: candidate_stability,
        source: candidate_source,
        confidence: candidate_confidence,
        created_at_ms: now_ms,
        projection: projection_state,
    })
}

pub async fn retrieve_memory_hits(
    app_handle: &AppHandle,
    query: String,
    top_k: usize,
) -> Result<Vec<crate::agents::RetrievedMemoryItem>> {
    Ok(retrieve_memory_outcome(app_handle, query, top_k)
        .await?
        .hits)
}

pub async fn retrieve_memory_outcome(
    app_handle: &AppHandle,
    query: String,
    top_k: usize,
) -> Result<MemoryRetrieveOutcome> {
    let mut state = ContextRunState::default();
    let request = MemoryQuery {
        execution_id: MEMORY_TOOL_EXECUTION_ID.to_string(),
        query,
        top_k: top_k.max(1),
        include_reflection: false,
        respect_auto_inject_suppression: false,
    };

    let retrieval = retrieve_memory_unified(app_handle, &mut state, &request).await;
    let trace = build_memory_retrieval_trace(
        &retrieval.query_used,
        request.top_k,
        &retrieval.hits,
        retrieval.used_canonical_fallback,
        request.include_reflection,
    );
    Ok(MemoryRetrieveOutcome {
        hits: retrieval.hits,
        trace,
    })
}

pub async fn retrieve_memory_items_structured(
    app_handle: &AppHandle,
    query: String,
    top_k: usize,
) -> Result<MemoryManagerRetrieveResult> {
    let outcome = retrieve_memory_outcome(app_handle, query, top_k).await?;
    Ok(MemoryManagerRetrieveResult {
        items: outcome
            .hits
            .into_iter()
            .map(|item| MemoryManagerResultItem {
                id: item.id,
                text: item.text,
                kind: item.kind,
                scope: item.scope,
                stability: item.stability,
                source: item.source,
                confidence: item.confidence,
                importance: item.importance,
                created_at_ms: item.created_at_ms,
                score: item.score,
            })
            .collect(),
        trace: map_tool_trace(outcome.trace),
    })
}

fn map_tool_trace(trace: MemoryRetrievalTrace) -> MemoryManagerRetrievalTrace {
    MemoryManagerRetrievalTrace {
        query_preview: trace.query_preview,
        requested_top_k: trace.requested_top_k,
        hit_count: trace.hit_count,
        used_canonical_fallback: trace.used_canonical_fallback,
        include_reflection: trace.include_reflection,
        source_breakdown: trace
            .source_breakdown
            .into_iter()
            .map(|item| MemoryManagerTraceCount {
                label: item.label,
                count: item.count,
            })
            .collect(),
        kind_breakdown: trace
            .kind_breakdown
            .into_iter()
            .map(|item| MemoryManagerTraceCount {
                label: item.label,
                count: item.count,
            })
            .collect(),
    }
}

pub fn map_store_result(outcome: MemoryStoreOutcome) -> MemoryManagerStoreResult {
    MemoryManagerStoreResult {
        memory_id: outcome.memory_id.clone(),
        title: outcome.title,
        kind: outcome.kind,
        scope: outcome.scope,
        stability: outcome.stability,
        source: outcome.source,
        confidence: outcome.confidence,
        created_at_ms: outcome.created_at_ms,
        projection: MemoryManagerProjectionState {
            memory_id: outcome.memory_id,
            lexical_indexed: outcome.projection.lexical_indexed,
            vector_indexed: outcome.projection.vector_indexed,
            skill_projected: outcome.projection.skill_projected,
            last_error: outcome.projection.last_error,
            updated_at_ms: outcome.projection.updated_at_ms,
        },
    }
}
