//! Reflection memory — auto-generates reflections after task execution
//! and persists them as high-priority memory items for future recall.

use std::collections::HashMap;
use std::sync::Arc;
use tauri::{AppHandle, Manager};

use crate::agents::context_engineering::checkpoint::{
    load_run_state, save_run_state, ContextMemoryItem,
};

const MAX_REFLECTION_LENGTH: usize = 300;
const REFLECTION_IMPORTANCE: u8 = 5;

/// Summary of a completed execution for reflection generation.
#[derive(Debug, Clone)]
pub struct ExecutionOutcome {
    pub execution_id: String,
    pub task: String,
    pub success: bool,
    pub error: Option<String>,
    pub tool_names_used: Vec<String>,
    pub response_excerpt: Option<String>,
}

/// Generate and store a reflection memory item after execution.
/// Only generates reflections for failures or partial failures.
pub async fn record_execution_reflection(
    app_handle: &AppHandle,
    outcome: &ExecutionOutcome,
) {
    if outcome.success && outcome.error.is_none() {
        // Successful executions: only reflect if noteworthy tools were used
        if outcome.tool_names_used.len() < 3 {
            return;
        }
        let reflection = build_success_reflection(outcome);
        if !reflection.is_empty() {
            persist_reflection(app_handle, &outcome.execution_id, &reflection).await;
        }
        return;
    }

    let reflection = build_failure_reflection(outcome);
    if !reflection.is_empty() {
        persist_reflection(app_handle, &outcome.execution_id, &reflection).await;
        persist_reflection_to_vector_store(app_handle, &reflection).await;
    }
}

fn build_failure_reflection(outcome: &ExecutionOutcome) -> String {
    let task_brief = truncate_str(&outcome.task, 120);
    let error_brief = outcome
        .error
        .as_deref()
        .map(|e| truncate_str(e, 120))
        .unwrap_or_default();

    let tools_used = if outcome.tool_names_used.is_empty() {
        "none".to_string()
    } else {
        outcome.tool_names_used.join(", ")
    };

    let mut reflection = format!(
        "REFLECTION(failure): Task '{}' failed.",
        task_brief
    );

    if !error_brief.is_empty() {
        reflection.push_str(&format!(" Error: {}.", error_brief));
    }

    reflection.push_str(&format!(" Tools used: {}.", tools_used));

    // Add actionable hints based on common error patterns
    if let Some(ref err) = outcome.error {
        let err_lower = err.to_lowercase();
        if err_lower.contains("timeout") || err_lower.contains("timed out") {
            reflection.push_str(" Consider using longer timeouts or breaking into smaller steps.");
        } else if err_lower.contains("permission") || err_lower.contains("denied") {
            reflection.push_str(" Check permissions or use elevated execution mode.");
        } else if err_lower.contains("not found") || err_lower.contains("no such file") {
            reflection.push_str(" Verify paths and file existence before operations.");
        } else if err_lower.contains("connection") || err_lower.contains("network") {
            reflection.push_str(" Network issue — retry or check connectivity.");
        } else if err_lower.contains("parse") || err_lower.contains("invalid") {
            reflection.push_str(" Validate input format before processing.");
        }
    }

    truncate_str(&reflection, MAX_REFLECTION_LENGTH).to_string()
}

fn build_success_reflection(outcome: &ExecutionOutcome) -> String {
    let task_brief = truncate_str(&outcome.task, 100);
    let tools = outcome.tool_names_used.join(", ");

    let reflection = format!(
        "REFLECTION(success): Task '{}' completed successfully using tools: {}.",
        task_brief, tools
    );

    truncate_str(&reflection, MAX_REFLECTION_LENGTH).to_string()
}

async fn persist_reflection(
    app_handle: &AppHandle,
    execution_id: &str,
    reflection_text: &str,
) {
    let mut state = match load_run_state(app_handle, execution_id).await {
        Ok(Some(s)) => s,
        Ok(None) => return,
        Err(e) => {
            tracing::warn!("Failed to load run state for reflection: {}", e);
            return;
        }
    };

    // Avoid duplicate reflections
    if state
        .memory_items
        .iter()
        .any(|item| item.kind == "reflection" && item.text == reflection_text)
    {
        return;
    }

    let now_ms = chrono::Utc::now().timestamp_millis();
    state.memory_items.push(ContextMemoryItem {
        id: uuid::Uuid::new_v4().to_string(),
        text: reflection_text.to_string(),
        kind: "reflection".to_string(),
        importance: REFLECTION_IMPORTANCE,
        created_at_ms: now_ms,
        last_used_at_ms: now_ms,
    });

    state.run_state_version += 1;
    state.last_updated_at_ms = now_ms;

    if let Err(e) = save_run_state(app_handle, execution_id, &state).await {
        tracing::warn!("Failed to save reflection to run state: {}", e);
    } else {
        tracing::info!(
            "Reflection recorded for execution {}: {}",
            execution_id,
            truncate_str(reflection_text, 80)
        );
    }
}

async fn persist_reflection_to_vector_store(app_handle: &AppHandle, reflection_text: &str) {
    let db = match app_handle.try_state::<Arc<sentinel_db::DatabaseService>>() {
        Some(db) => db,
        None => return,
    };

    let rag_service = match crate::commands::rag_commands::get_or_init_rag_service(
        db.inner().clone(),
    )
    .await
    {
        Ok(s) => s,
        Err(e) => {
            tracing::debug!("RAG service unavailable for reflection persistence: {}", e);
            return;
        }
    };

    let collection_id =
        match crate::commands::rag_commands::ensure_memory_collection_exists(db.inner().clone())
            .await
        {
            Ok(id) => id,
            Err(e) => {
                tracing::debug!("Memory collection unavailable: {}", e);
                return;
            }
        };

    let title = format!(
        "[reflection] {}",
        truncate_str(reflection_text, 80)
    );
    let mut metadata = HashMap::new();
    metadata.insert("type".to_string(), "agent_memory".to_string());
    metadata.insert("kind".to_string(), "reflection".to_string());
    metadata.insert(
        "created_at".to_string(),
        chrono::Utc::now().timestamp_millis().to_string(),
    );

    if let Err(e) = rag_service
        .ingest_text(&title, reflection_text, Some(&collection_id), Some(metadata))
        .await
    {
        tracing::warn!("Failed to persist reflection to vector store: {}", e);
    }
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
