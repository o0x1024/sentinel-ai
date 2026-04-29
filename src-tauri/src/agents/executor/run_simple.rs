//! Simple execution path without tools.

use anyhow::Result;
use std::sync::{Arc, Mutex};
use tauri::{AppHandle, Emitter, Manager};

use sentinel_llm::{LlmConfig, StreamContent, StreamingLlmClient};

use super::AgentExecuteParams;
use crate::agents::apply_sentinel_execution_outcome;
use crate::agents::executor::message_store::{
    build_assistant_session_stats_metadata, mark_first_response_ms, save_assistant_message,
};
use crate::agents::executor::utils::cleanup_container_context_async;
use crate::utils::ai_generation_settings::apply_generation_settings_from_db;

pub async fn execute_agent_simple(
    app_handle: &AppHandle,
    params: AgentExecuteParams,
) -> Result<String> {
    let execution_started_at_ms = chrono::Utc::now().timestamp_millis();
    let rig_provider = params.rig_provider.to_lowercase();

    let mut config = LlmConfig::new(&rig_provider, &params.model)
        .with_timeout(params.timeout_secs)
        .with_rig_provider(&rig_provider)
        .with_conversation_id(&params.execution_id);

    if let Some(ref api_key) = params.api_key {
        config = config.with_api_key(api_key);
    }

    if let Some(ref api_base) = params.api_base {
        config = config.with_base_url(api_base);
    }

    if let Some(db) = app_handle.try_state::<Arc<sentinel_db::DatabaseService>>() {
        config = apply_generation_settings_from_db(db.as_ref(), config).await;
    }

    let system_prompt = params.system_prompt.clone();
    let client = StreamingLlmClient::new(config);
    let execution_id = params.execution_id.clone();
    let app = app_handle.clone();
    let reasoning_content = Arc::new(Mutex::new(String::new()));
    let reasoning_content_for_stream = reasoning_content.clone();
    let usage_data = Arc::new(Mutex::new(None::<(u32, u32)>));
    let usage_data_for_stream = usage_data.clone();
    let first_response_ms = Arc::new(Mutex::new(None::<i64>));
    let first_response_ms_for_stream = first_response_ms.clone();

    let result = client
        .stream_completion(Some(&system_prompt), &params.task, |content| {
            if crate::commands::ai::is_conversation_cancelled(&execution_id) {
                return false;
            }
            match content {
                StreamContent::Text(text) => {
                    mark_first_response_ms(
                        first_response_ms_for_stream.as_ref(),
                        execution_started_at_ms,
                    );
                    let _ = app.emit(
                        "agent:chunk",
                        &serde_json::json!({
                            "execution_id": execution_id,
                            "chunk_type": "text",
                            "content": text,
                        }),
                    );
                }
                StreamContent::Reasoning(reasoning) => {
                    mark_first_response_ms(
                        first_response_ms_for_stream.as_ref(),
                        execution_started_at_ms,
                    );
                    if let Ok(mut buf) = reasoning_content_for_stream.lock() {
                        buf.push_str(&reasoning);
                    }
                    let _ = app.emit(
                        "agent:chunk",
                        &serde_json::json!({
                            "execution_id": execution_id,
                            "chunk_type": "reasoning",
                            "content": reasoning,
                        }),
                    );
                }
                StreamContent::Done => {
                    tracing::info!("Agent completed - execution_id: {}", execution_id);
                }
                StreamContent::Usage {
                    input_tokens,
                    output_tokens,
                } => {
                    if let Ok(mut guard) = usage_data_for_stream.lock() {
                        *guard = Some((input_tokens, output_tokens));
                    }
                }
                _ => {}
            }
            true
        })
        .await;

    match result {
        Ok(response) => {
            tracing::info!(
                "Agent execution successful - execution_id: {}, response_length: {}",
                params.execution_id,
                response.len()
            );

            let final_reasoning_content = reasoning_content
                .lock()
                .ok()
                .map(|buf| buf.clone())
                .filter(|buf| !buf.trim().is_empty());
            let (input_tokens, output_tokens) = if let Ok(guard) = usage_data.lock() {
                guard.unwrap_or((0, 0))
            } else {
                (0, 0)
            };
            let session_metadata = build_assistant_session_stats_metadata(
                Some(chrono::Utc::now().timestamp_millis() - execution_started_at_ms),
                first_response_ms.lock().ok().and_then(|guard| *guard),
                Some(input_tokens),
                Some(output_tokens),
            );

            save_assistant_message(
                app_handle,
                &params.execution_id,
                &response,
                None,
                final_reasoning_content,
                session_metadata,
                params.persist_messages,
                params.subagent_run_id.as_deref(),
            )
            .await;
            if let Err(err) = apply_sentinel_execution_outcome(
                app_handle,
                &params.execution_id,
                true,
                Some(&response),
                None,
            )
            .await
            {
                tracing::warn!("Failed to update sentinel execution outcome: {}", err);
            }

            cleanup_container_context_async(app_handle, &params.execution_id).await;
            Ok(response)
        }
        Err(e) => {
            tracing::error!(
                "Agent execution failed - execution_id: {}, error: {}",
                params.execution_id,
                e
            );
            if let Err(update_err) = apply_sentinel_execution_outcome(
                app_handle,
                &params.execution_id,
                false,
                None,
                Some(&e.to_string()),
            )
            .await
            {
                tracing::warn!(
                    "Failed to update sentinel execution outcome after error: {}",
                    update_err
                );
            }
            cleanup_container_context_async(app_handle, &params.execution_id).await;
            Err(e)
        }
    }
}
