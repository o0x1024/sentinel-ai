use anyhow::{anyhow, Result};
use std::collections::HashMap;
use std::sync::Arc;
use tauri::{AppHandle, Manager};

use crate::agents::context_engineering::{
    retrieve_memory_unified, ContextRunState, MemoryQuery, RetrievedMemoryItem,
};
use crate::commands::rag_commands::{ensure_memory_collection_exists, get_or_init_rag_service};
use crate::memory::{build_memory_retrieval_trace, MemoryRetrieveOutcome};
use crate::commands::rag_commands::MEMORY_COLLECTION_NAME;
use sentinel_db::core::models::database::{DurableMemoryProjectionState, DurableMemoryRecord};

pub const NO_AUTO_INJECT_TAG: &str = "no_auto_inject";

#[derive(Debug, Clone, serde::Serialize)]
pub struct DurableMemoryUpdateResult {
    pub record: DurableMemoryRecord,
    pub projection: DurableMemoryProjectionState,
}

#[derive(Debug, Clone, serde::Serialize)]
pub struct DurableMemoryDeleteResult {
    pub memory_id: String,
    pub archived: bool,
    pub lexical_removed: bool,
    pub vector_documents_removed: usize,
}

#[derive(Debug, Clone, serde::Serialize)]
pub struct DurableMemoryAutoInjectResult {
    pub memory_id: String,
    pub auto_inject_enabled: bool,
    pub record: DurableMemoryRecord,
}

fn parse_tags_json(raw: &str) -> Vec<String> {
    serde_json::from_str(raw).unwrap_or_default()
}

fn write_tags_json(tags: &[String]) -> String {
    serde_json::to_string(tags).unwrap_or_else(|_| "[]".to_string())
}

pub fn memory_auto_inject_enabled(record: &DurableMemoryRecord) -> bool {
    record.status == "active"
        && !parse_tags_json(record.tags_json.as_str()).contains(&NO_AUTO_INJECT_TAG.to_string())
}

pub async fn filter_hits_for_auto_inject(
    app_handle: &AppHandle,
    hits: Vec<RetrievedMemoryItem>,
) -> Result<Vec<RetrievedMemoryItem>> {
    let db = app_handle
        .try_state::<Arc<sentinel_db::DatabaseService>>()
        .ok_or_else(|| anyhow!("DatabaseService not available"))?
        .inner()
        .clone();

    let mut filtered = Vec::with_capacity(hits.len());
    for hit in hits {
        let Some(record) = db
            .get_durable_memory_record_internal(&hit.id)
            .await
            .map_err(|e| anyhow!(e.to_string()))?
        else {
            filtered.push(hit);
            continue;
        };
        if memory_auto_inject_enabled(&record) {
            filtered.push(hit);
        }
    }
    Ok(filtered)
}

async fn remove_memory_projections(
    app_handle: &AppHandle,
    memory_id: &str,
) -> Result<(bool, usize)> {
    let db = app_handle
        .try_state::<Arc<sentinel_db::DatabaseService>>()
        .ok_or_else(|| anyhow!("DatabaseService not available"))?
        .inner()
        .clone();

    let mut lexical_removed = false;
    let mut vector_removed = 0usize;
    let mut errors = Vec::new();

    if let Ok(service) = get_or_init_rag_service(db.clone()).await {
        match service.delete_memory_lexical_document(memory_id).await {
            Ok(removed) => lexical_removed = removed,
            Err(error) => errors.push(format!("lexical delete failed: {error}")),
        }
        match service
            .delete_memory_vector_projections(MEMORY_COLLECTION_NAME, memory_id)
            .await
        {
            Ok(count) => vector_removed = count,
            Err(error) => errors.push(format!("vector delete failed: {error}")),
        }
    } else {
        errors.push("rag service unavailable".to_string());
    }

    let now_ms = chrono::Utc::now().timestamp_millis();
    let projection = DurableMemoryProjectionState {
        memory_id: memory_id.to_string(),
        lexical_indexed: false,
        vector_indexed: false,
        skill_projected: false,
        last_error: (!errors.is_empty()).then(|| errors.join(" | ")),
        updated_at_ms: now_ms,
    };
    db.upsert_durable_memory_projection_state_internal(&projection)
        .await
        .map_err(|e| anyhow!(e.to_string()))?;

    Ok((lexical_removed, vector_removed))
}

async fn reproject_memory_record(
    app_handle: &AppHandle,
    record: &DurableMemoryRecord,
    tags: &[String],
) -> Result<DurableMemoryProjectionState> {
    let db = app_handle
        .try_state::<Arc<sentinel_db::DatabaseService>>()
        .ok_or_else(|| anyhow!("DatabaseService not available"))?
        .inner()
        .clone();

    let title = record
        .title
        .clone()
        .unwrap_or_else(|| format!("[{}] {}", record.kind, record.text.chars().take(30).collect::<String>()));

    let durable_metadata = sentinel_rag::build_memory_durable_metadata(
        Some(record.scope.as_str()),
        None,
        Some(record.source.as_str()),
        None,
        record.kind.as_str(),
        tags,
    );

    let mut metadata = HashMap::new();
    metadata.insert("type".to_string(), "agent_memory".to_string());
    metadata.insert("kind".to_string(), record.kind.clone());
    metadata.insert("scope".to_string(), durable_metadata.scope.clone());
    metadata.insert("stability".to_string(), durable_metadata.stability.clone());
    metadata.insert("source".to_string(), durable_metadata.source.clone());
    metadata.insert(
        "confidence".to_string(),
        format!("{:.2}", durable_metadata.confidence),
    );
    metadata.insert("memory_id".to_string(), record.id.clone());
    if !tags.is_empty() {
        metadata.insert("tags".to_string(), tags.join(","));
    }

    let mut projection_state = DurableMemoryProjectionState {
        memory_id: record.id.clone(),
        lexical_indexed: false,
        vector_indexed: false,
        skill_projected: false,
        last_error: None,
        updated_at_ms: chrono::Utc::now().timestamp_millis(),
    };

    let mut errors = Vec::new();
    if let Ok(service) = get_or_init_rag_service(db.clone()).await {
        let _ = service
            .delete_memory_vector_projections(MEMORY_COLLECTION_NAME, record.id.as_str())
            .await;

        match ensure_memory_collection_exists(db.clone()).await {
            Ok(collection_id) => {
                if let Err(error) = service
                    .ingest_text(&title, record.text.as_str(), Some(&collection_id), Some(metadata))
                    .await
                {
                    errors.push(format!("vector projection failed: {error}"));
                } else {
                    projection_state.vector_indexed = true;
                }
            }
            Err(error) => errors.push(format!("memory collection unavailable: {error}")),
        }

        if let Err(error) = service
            .upsert_memory_lexical_document(sentinel_rag::MemoryLexicalDocument {
                id: record.id.clone(),
                collection_name: MEMORY_COLLECTION_NAME.to_string(),
                title: Some(title),
                body: record.text.clone(),
                normalized_text: sentinel_rag::normalize_memory_text(record.text.as_str()),
                identifiers: sentinel_rag::extract_memory_identifiers(record.text.as_str()),
                tags: tags.join(","),
                kind: record.kind.clone(),
                scope: durable_metadata.scope,
                stability: durable_metadata.stability,
                source: durable_metadata.source,
                confidence: durable_metadata.confidence,
                importance: record.importance.clamp(1, 5) as u8,
                created_at_ms: record.created_at_ms,
                updated_at_ms: chrono::Utc::now().timestamp_millis(),
            })
            .await
        {
            errors.push(format!("lexical projection failed: {error}"));
        } else {
            projection_state.lexical_indexed = true;
        }
    } else {
        errors.push("rag service unavailable".to_string());
    }

    projection_state.last_error = (!errors.is_empty()).then(|| errors.join(" | "));
    projection_state.updated_at_ms = chrono::Utc::now().timestamp_millis();
    db.upsert_durable_memory_projection_state_internal(&projection_state)
        .await
        .map_err(|e| anyhow!(e.to_string()))?;

    Ok(projection_state)
}

pub async fn update_durable_memory(
    app_handle: &AppHandle,
    memory_id: String,
    text: String,
    title: Option<String>,
    kind: Option<String>,
    tags: Option<Vec<String>>,
) -> Result<DurableMemoryUpdateResult> {
    let trimmed = text.trim();
    if trimmed.is_empty() {
        return Err(anyhow!("Memory text cannot be empty"));
    }

    let db = app_handle
        .try_state::<Arc<sentinel_db::DatabaseService>>()
        .ok_or_else(|| anyhow!("DatabaseService not available"))?
        .inner()
        .clone();

    let Some(mut record) = db
        .get_durable_memory_record_internal(memory_id.trim())
        .await
        .map_err(|e| anyhow!(e.to_string()))?
    else {
        return Err(anyhow!("Memory record not found"));
    };

    if record.status == "archived" {
        return Err(anyhow!("Cannot update archived memory"));
    }

    record.text = trimmed.to_string();
    if let Some(next_title) = title {
        let title_trimmed = next_title.trim();
        record.title = if title_trimmed.is_empty() {
            None
        } else {
            Some(title_trimmed.to_string())
        };
    }
    if let Some(next_kind) = kind {
        let kind_trimmed = next_kind.trim();
        if !kind_trimmed.is_empty() {
            record.kind = kind_trimmed.to_string();
        }
    }
    if let Some(next_tags) = tags {
        record.tags_json = write_tags_json(&next_tags);
    }
    record.updated_at_ms = chrono::Utc::now().timestamp_millis();

    db.upsert_durable_memory_record_internal(&record)
        .await
        .map_err(|e| anyhow!(e.to_string()))?;

    let tag_items = parse_tags_json(record.tags_json.as_str());
    let projection = reproject_memory_record(app_handle, &record, &tag_items).await?;

    Ok(DurableMemoryUpdateResult { record, projection })
}

pub async fn delete_durable_memory(
    app_handle: &AppHandle,
    memory_id: String,
) -> Result<DurableMemoryDeleteResult> {
    let db = app_handle
        .try_state::<Arc<sentinel_db::DatabaseService>>()
        .ok_or_else(|| anyhow!("DatabaseService not available"))?
        .inner()
        .clone();

    let memory_id = memory_id.trim().to_string();
    let Some(mut record) = db
        .get_durable_memory_record_internal(&memory_id)
        .await
        .map_err(|e| anyhow!(e.to_string()))?
    else {
        return Err(anyhow!("Memory record not found"));
    };

    record.status = "archived".to_string();
    record.updated_at_ms = chrono::Utc::now().timestamp_millis();
    db.upsert_durable_memory_record_internal(&record)
        .await
        .map_err(|e| anyhow!(e.to_string()))?;

    let (lexical_removed, vector_documents_removed) =
        remove_memory_projections(app_handle, &memory_id).await?;

    Ok(DurableMemoryDeleteResult {
        memory_id,
        archived: true,
        lexical_removed,
        vector_documents_removed,
    })
}

pub async fn set_durable_memory_auto_inject(
    app_handle: &AppHandle,
    memory_id: String,
    enabled: bool,
) -> Result<DurableMemoryAutoInjectResult> {
    let db = app_handle
        .try_state::<Arc<sentinel_db::DatabaseService>>()
        .ok_or_else(|| anyhow!("DatabaseService not available"))?
        .inner()
        .clone();

    let memory_id = memory_id.trim().to_string();
    let Some(mut record) = db
        .get_durable_memory_record_internal(&memory_id)
        .await
        .map_err(|e| anyhow!(e.to_string()))?
    else {
        return Err(anyhow!("Memory record not found"));
    };

    if record.status == "archived" {
        return Err(anyhow!("Cannot update archived memory"));
    }

    let mut tags = parse_tags_json(record.tags_json.as_str());
    if enabled {
        tags.retain(|tag| tag != NO_AUTO_INJECT_TAG);
    } else if !tags.iter().any(|tag| tag == NO_AUTO_INJECT_TAG) {
        tags.push(NO_AUTO_INJECT_TAG.to_string());
    }
    record.tags_json = write_tags_json(&tags);
    record.updated_at_ms = chrono::Utc::now().timestamp_millis();
    db.upsert_durable_memory_record_internal(&record)
        .await
        .map_err(|e| anyhow!(e.to_string()))?;

    Ok(DurableMemoryAutoInjectResult {
        memory_id: record.id.clone(),
        auto_inject_enabled: memory_auto_inject_enabled(&record),
        record,
    })
}

pub async fn preview_memory_retrieval(
    app_handle: &AppHandle,
    query: String,
    top_k: usize,
) -> Result<MemoryRetrieveOutcome> {
    let mut state = ContextRunState::default();
    let request = MemoryQuery {
        execution_id: "memory_preview".to_string(),
        query,
        top_k: top_k.max(1),
        include_reflection: false,
        respect_auto_inject_suppression: true,
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
