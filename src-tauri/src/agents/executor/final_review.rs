use std::sync::{Arc, Mutex};

use sentinel_db::Database;
use serde_json::json;
use tauri::{AppHandle, Emitter};

use super::run_with_tools_support::{
    final_response_needs_evidence_review, final_response_needs_verification_review,
};
use super::tenth_man_hypothesis::HypothesisTracker;
use super::types::ToolCallRecord;
use super::AgentExecuteParams;
use crate::agents::tenth_man::{InterventionMode, TenthMan, TenthManTriggerPolicy};

pub(super) async fn run_final_tenth_man_review(
    app: &AppHandle,
    params: &AgentExecuteParams,
    db_for_stream: Option<Arc<sentinel_db::DatabaseService>>,
    final_response: &str,
    all_tool_calls: &[ToolCallRecord],
    hypothesis_tracker: &Arc<Mutex<HypothesisTracker>>,
    trigger_policy: &TenthManTriggerPolicy,
) {
    if !params.enable_tenth_man_rule {
        return;
    }

    let tenth_man = TenthMan::new(params);
    let final_focus_hint = hypothesis_tracker
        .lock()
        .ok()
        .and_then(|tracker| tracker.focus_hint().map(str::to_string));
    let requires_verification_review = trigger_policy.review_final_response_without_verification
        && final_response_needs_verification_review(final_response, all_tool_calls);
    let requires_evidence_review = trigger_policy.review_low_evidence_high_confidence
        && final_response_needs_evidence_review(
            final_response,
            final_focus_hint.as_deref(),
            all_tool_calls,
            trigger_policy.minimum_evidence_score(),
        );

    let mut final_trigger = "final_review";
    let should_run_final = if let Some(ref config) = params.tenth_man_config {
        match &config.mode {
            InterventionMode::SystemOnly => true,
            InterventionMode::Hybrid {
                force_final_review, ..
            } => {
                if requires_evidence_review && requires_verification_review {
                    final_trigger = "final_response_low_evidence_and_unverified";
                    true
                } else if requires_evidence_review {
                    final_trigger = "final_response_low_evidence_high_confidence";
                    true
                } else if requires_verification_review {
                    final_trigger = "final_response_without_verification";
                    true
                } else {
                    *force_final_review
                }
            }
            InterventionMode::ToolOnly => false,
            _ => {
                if requires_evidence_review && requires_verification_review {
                    final_trigger = "final_response_low_evidence_and_unverified";
                } else if requires_evidence_review {
                    final_trigger = "final_response_low_evidence_high_confidence";
                } else if requires_verification_review {
                    final_trigger = "final_response_without_verification";
                }
                true
            }
        }
    } else {
        if requires_evidence_review && requires_verification_review {
            final_trigger = "final_response_low_evidence_and_unverified";
        } else if requires_evidence_review {
            final_trigger = "final_response_low_evidence_high_confidence";
        } else if requires_verification_review {
            final_trigger = "final_response_without_verification";
        }
        true
    };

    if !should_run_final {
        tracing::info!("Skipping final Tenth Man review (mode: ToolOnly)");
        return;
    }

    tracing::info!(
        "Running Tenth Man final review with full history for execution_id: {} (trigger={})",
        params.execution_id,
        final_trigger
    );

    match tenth_man.review_with_history(&params.execution_id).await {
        Ok(critique) => {
            tracing::info!("Tenth Man Critique generated ({} chars)", critique.len());
            persist_and_emit_final_review(app, params, db_for_stream, final_trigger, critique)
                .await;
        }
        Err(error) => {
            tracing::warn!("Tenth Man Review failed: {}", error);
        }
    }
}

async fn persist_and_emit_final_review(
    app: &AppHandle,
    params: &AgentExecuteParams,
    db_for_stream: Option<Arc<sentinel_db::DatabaseService>>,
    final_trigger: &str,
    critique: String,
) {
    if !params.persist_messages {
        return;
    }
    let Some(db) = db_for_stream else {
        return;
    };
    use sentinel_core::models::database as core_db;
    let conversation_id = params.storage_conversation_id();
    let review_msg = core_db::AiMessage {
        id: uuid::Uuid::new_v4().to_string(),
        conversation_id: conversation_id.to_string(),
        role: "system".to_string(),
        content: critique.clone(),
        metadata: Some(
            json!({
                "kind": "tenth_man_critique",
                "trigger": final_trigger,
                "mode": "system_enforced"
            })
            .to_string(),
        ),
        token_count: Some(critique.len() as i32),
        cost: None,
        tool_calls: None,
        attachments: None,
        reasoning_content: None,
        timestamp: chrono::Utc::now(),
        architecture_type: None,
        architecture_meta: None,
        structured_data: None,
    };

    if let Err(error) = db.create_ai_message(&review_msg).await {
        tracing::warn!("Failed to save Tenth Man critique: {}", error);
    }

    let _ = app.emit(
        "agent:tenth_man_critique",
        &json!({
            "execution_id": params.execution_id,
            "generation": params.cancellation_generation,
            "critique": critique,
            "message_id": review_msg.id,
            "trigger": final_trigger,
            "mode": "system_enforced"
        }),
    );
}
