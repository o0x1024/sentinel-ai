//! Simple execution path without tools.

use anyhow::Result;
use tauri::{AppHandle, Emitter};

use sentinel_llm::{LlmConfig, StreamContent, StreamingLlmClient};

use crate::agents::executor::message_store::save_assistant_message;
use crate::agents::executor::utils::cleanup_container_context_async;
use super::AgentExecuteParams;

pub async fn execute_agent_simple(
    app_handle: &AppHandle,
    params: AgentExecuteParams,
) -> Result<String> {
    let rig_provider = params.rig_provider.to_lowercase();

    let mut config = LlmConfig::new(&rig_provider, &params.model)
        .with_timeout(params.timeout_secs)
        .with_rig_provider(&rig_provider);

    if let Some(ref api_key) = params.api_key {
        config = config.with_api_key(api_key);
    }

    if let Some(ref api_base) = params.api_base {
        config = config.with_base_url(api_base);
    }

    let system_prompt = params.system_prompt.clone();
    let client = StreamingLlmClient::new(config);
    let execution_id = params.execution_id.clone();
    let app = app_handle.clone();

    let result = client
        .stream_completion(
            Some(&system_prompt),
            &params.task,
            |content| {
                if crate::commands::ai::is_conversation_cancelled(&execution_id) {
                    return false;
                }
                match content {
                    StreamContent::Text(text) => {
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
                    _ => {}
                }
                true
            },
        )
        .await;

    match result {
        Ok(response) => {
            tracing::info!(
                "Agent execution successful - execution_id: {}, response_length: {}",
                params.execution_id,
                response.len()
            );

            save_assistant_message(
                app_handle,
                &params.execution_id,
                &response,
                None,
                None,
                params.persist_messages,
                params.subagent_run_id.as_deref(),
            )
            .await;

            cleanup_container_context_async(&params.execution_id).await;
            Ok(response)
        }
        Err(e) => {
            tracing::error!(
                "Agent execution failed - execution_id: {}, error: {}",
                params.execution_id,
                e
            );
            cleanup_container_context_async(&params.execution_id).await;
            Err(e)
        }
    }
}

