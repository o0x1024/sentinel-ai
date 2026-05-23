use crate::services::ensure_bug_bounty_access;
use chrono::Utc;
use sentinel_db::{
    BountyKnowledgeNoteFilter, BountyKnowledgeNoteRow, BountyKnowledgeNoteSearchFilter,
    BountyKnowledgeNoteSearchRow, BountyKnowledgeNoteStats, DatabaseService,
};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tauri::State;
use uuid::Uuid;

fn normalize_optional_string(value: Option<String>) -> Option<String> {
    value.and_then(|value| {
        let trimmed = value.trim().to_string();
        if trimmed.is_empty() {
            None
        } else {
            Some(trimmed)
        }
    })
}

fn normalize_note_tags(tags: Option<Vec<String>>) -> Option<String> {
    tags.and_then(|tags| {
        let normalized: Vec<String> = tags
            .into_iter()
            .map(|tag| tag.trim().to_string())
            .filter(|tag| !tag.is_empty())
            .collect();

        if normalized.is_empty() {
            None
        } else {
            Some(serde_json::to_string(&normalized).unwrap_or_else(|_| "[]".to_string()))
        }
    })
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateBountyKnowledgeNoteRequest {
    pub program_id: Option<String>,
    pub title: String,
    pub content: String,
    pub tags: Option<Vec<String>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UpdateBountyKnowledgeNoteRequest {
    pub program_id: Option<String>,
    pub title: String,
    pub content: String,
    pub tags: Option<Vec<String>>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct BountyKnowledgeNoteListRequest {
    pub program_id: Option<String>,
    pub only_unbound: Option<bool>,
    pub limit: Option<u32>,
    pub offset: Option<u32>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BountyKnowledgeNoteSearchRequest {
    pub query: String,
    pub program_id: Option<String>,
    pub only_unbound: Option<bool>,
    pub limit: Option<u32>,
    pub offset: Option<u32>,
}

#[tauri::command]
pub async fn bounty_create_knowledge_note(
    db_service: State<'_, Arc<DatabaseService>>,
    request: CreateBountyKnowledgeNoteRequest,
) -> Result<BountyKnowledgeNoteRow, String> {
    ensure_bug_bounty_access()?;

    let title = request.title.trim().to_string();
    let content = request.content.trim().to_string();

    if title.is_empty() {
        return Err("Title is required".to_string());
    }
    if content.is_empty() {
        return Err("Content is required".to_string());
    }

    let now = Utc::now().to_rfc3339();
    let note = BountyKnowledgeNoteRow {
        id: Uuid::new_v4().to_string(),
        program_id: normalize_optional_string(request.program_id),
        title,
        content,
        tags_json: normalize_note_tags(request.tags),
        metadata_json: None,
        created_at: now.clone(),
        updated_at: now,
    };

    db_service
        .create_bounty_knowledge_note(&note)
        .await
        .map_err(|e| e.to_string())?;

    Ok(note)
}

#[tauri::command]
pub async fn bounty_get_knowledge_note(
    db_service: State<'_, Arc<DatabaseService>>,
    id: String,
) -> Result<Option<BountyKnowledgeNoteRow>, String> {
    db_service
        .get_bounty_knowledge_note(id.trim())
        .await
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn bounty_update_knowledge_note(
    db_service: State<'_, Arc<DatabaseService>>,
    id: String,
    request: UpdateBountyKnowledgeNoteRequest,
) -> Result<BountyKnowledgeNoteRow, String> {
    ensure_bug_bounty_access()?;

    let existing = db_service
        .get_bounty_knowledge_note(id.trim())
        .await
        .map_err(|e| e.to_string())?
        .ok_or_else(|| "Knowledge note not found".to_string())?;

    let title = request.title.trim().to_string();
    let content = request.content.trim().to_string();

    if title.is_empty() {
        return Err("Title is required".to_string());
    }
    if content.is_empty() {
        return Err("Content is required".to_string());
    }

    let updated = BountyKnowledgeNoteRow {
        id: existing.id,
        program_id: normalize_optional_string(request.program_id),
        title,
        content,
        tags_json: normalize_note_tags(request.tags),
        metadata_json: existing.metadata_json,
        created_at: existing.created_at,
        updated_at: Utc::now().to_rfc3339(),
    };

    db_service
        .update_bounty_knowledge_note(&updated)
        .await
        .map_err(|e| e.to_string())?;

    Ok(updated)
}

#[tauri::command]
pub async fn bounty_delete_knowledge_note(
    db_service: State<'_, Arc<DatabaseService>>,
    id: String,
) -> Result<bool, String> {
    ensure_bug_bounty_access()?;

    db_service
        .delete_bounty_knowledge_note(id.trim())
        .await
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn bounty_list_knowledge_notes(
    db_service: State<'_, Arc<DatabaseService>>,
    filter: Option<BountyKnowledgeNoteListRequest>,
) -> Result<Vec<BountyKnowledgeNoteSearchRow>, String> {
    let filter = filter.unwrap_or_default();

    db_service
        .list_bounty_knowledge_notes(BountyKnowledgeNoteFilter {
            program_id: filter.program_id.as_deref(),
            only_unbound: filter.only_unbound.unwrap_or(false),
            limit: filter.limit,
            offset: filter.offset,
        })
        .await
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn bounty_search_knowledge_notes(
    db_service: State<'_, Arc<DatabaseService>>,
    request: BountyKnowledgeNoteSearchRequest,
) -> Result<Vec<BountyKnowledgeNoteSearchRow>, String> {
    let query = request.query.trim().to_string();
    if query.is_empty() {
        return Ok(vec![]);
    }

    db_service
        .search_bounty_knowledge_notes(BountyKnowledgeNoteSearchFilter {
            query: &query,
            program_id: request.program_id.as_deref(),
            only_unbound: request.only_unbound.unwrap_or(false),
            limit: request.limit,
            offset: request.offset,
        })
        .await
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn bounty_get_knowledge_note_stats(
    db_service: State<'_, Arc<DatabaseService>>,
    program_id: Option<String>,
) -> Result<BountyKnowledgeNoteStats, String> {
    db_service
        .get_bounty_knowledge_note_stats(program_id.as_deref())
        .await
        .map_err(|e| e.to_string())
}
