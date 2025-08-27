use crate::services::ai::{StreamMessage, StreamError};
use anyhow::Result;
use serde_json::Value;
use tauri::{AppHandle, Emitter};
use tracing::{debug, warn, info};

/// Centralized stream event emission helper
pub struct StreamEventEmitter {
    app_handle: Option<AppHandle>,
    conversation_id: String,
    message_id: String,
}

impl StreamEventEmitter {
    pub fn new(
        app_handle: Option<AppHandle>,
        conversation_id: String,
        message_id: String,
    ) -> Self {
        Self {
            app_handle,
            conversation_id,
            message_id,
        }
    }

    /// Emit stream start event
    pub fn emit_stream_start(&self) -> Result<()> {
        if let Some(app_handle) = &self.app_handle {
            let start_message = serde_json::json!({
                "conversation_id": self.conversation_id,
                "message_id": self.message_id
            });
            
            info!("Emitting ai_stream_start with message_id: {} for conversation: {}", 
                self.message_id, self.conversation_id);
            
            app_handle.emit("ai_stream_start", &start_message)
                .map_err(|e| anyhow::anyhow!("Failed to emit stream start: {}", e))?;
            
            debug!("Successfully emitted ai_stream_start event");
        }
        Ok(())
    }

    /// Emit incremental stream message
    pub fn emit_stream_chunk(&self, content_delta: &str, total_content: &str) -> Result<()> {
        if let Some(app_handle) = &self.app_handle {
            let stream_message = StreamMessage {
                conversation_id: self.conversation_id.clone(),
                message_id: self.message_id.clone(),
                content: content_delta.to_string(),
                is_complete: false,
                token_count: None,
                total_tokens: None,
                tool_calls: None,
                is_incremental: true,
                content_delta: Some(content_delta.to_string()),
                total_content_length: Some(total_content.len()),
                intent_type: None,
                stream_phase: Some("content".to_string()),
            };
            
            app_handle.emit("ai_stream_message", &stream_message)
                .map_err(|e| anyhow::anyhow!("Failed to emit stream chunk: {}", e))?;
                
            debug!("Emitted stream chunk: '{}' chars, total: {} chars", 
                   content_delta.len(), total_content.len());
        }
        Ok(())
    }

    /// Emit stream completion
    pub fn emit_stream_complete(&self, final_content: &str, total_tokens: Option<u32>) -> Result<()> {
        if let Some(app_handle) = &self.app_handle {
            // Emit final stream message
            let final_message = StreamMessage {
                conversation_id: self.conversation_id.clone(),
                message_id: self.message_id.clone(),
                content: final_content.to_string(),
                is_complete: true,
                token_count: None,
                total_tokens,
                tool_calls: None,
                is_incremental: false,
                content_delta: None,
                total_content_length: Some(final_content.len()),
                intent_type: None,
                stream_phase: Some("completion".to_string()),
            };
            
            app_handle.emit("ai_stream_message", &final_message)
                .map_err(|e| anyhow::anyhow!("Failed to emit final stream message: {}", e))?;
            
            // Emit stream complete event
            let complete_message = serde_json::json!({
                "conversation_id": self.conversation_id,
                "message_id": self.message_id,
                "total_tokens": total_tokens,
                "total_content_length": final_content.len()
            });
            
            app_handle.emit("ai_stream_complete", &complete_message)
                .map_err(|e| anyhow::anyhow!("Failed to emit stream complete: {}", e))?;
            
            info!("Stream completed for message_id: {}, content length: {}", 
                  self.message_id, final_content.len());
        }
        Ok(())
    }

    /// Emit stream error
    pub fn emit_stream_error(&self, error: &str, error_type: &str) -> Result<()> {
        if let Some(app_handle) = &self.app_handle {
            let error_message = StreamError {
                conversation_id: self.conversation_id.clone(),
                message_id: Some(self.message_id.clone()),
                execution_id: None,
                error: error.to_string(),
                error_type: error_type.to_string(),
            };
            
            app_handle.emit("ai_stream_error", &error_message)
                .map_err(|e| anyhow::anyhow!("Failed to emit stream error: {}", e))?;
            
            // Also emit stream complete to ensure frontend resets
            let complete_message = serde_json::json!({
                "conversation_id": self.conversation_id,
                "message_id": self.message_id,
                "total_content_length": 0,
                "error": true
            });
            let _ = app_handle.emit("ai_stream_complete", &complete_message);
            
            warn!("Emitted stream error: {} for message_id: {}", error, self.message_id);
        }
        Ok(())
    }

    /// Emit empty response with proper handling
    pub fn emit_empty_response(&self) -> Result<()> {
        if let Some(app_handle) = &self.app_handle {
            let empty_response_message = StreamMessage {
                conversation_id: self.conversation_id.clone(),
                message_id: self.message_id.clone(),
                content: String::new(),
                is_complete: true,
                token_count: None,
                total_tokens: None,
                tool_calls: None,
                is_incremental: false,
                content_delta: None,
                total_content_length: Some(0),
                intent_type: None,
                stream_phase: Some("content".to_string()),
            };
            
            app_handle.emit("ai_stream_message", &empty_response_message)
                .map_err(|e| anyhow::anyhow!("Failed to emit empty response: {}", e))?;
            
            debug!("Emitted empty response for message_id: {}", self.message_id);
        }
        Ok(())
    }

    /// Emit tool execution events
    pub fn emit_tool_execution_start(&self, tool_calls: &[crate::ai_adapter::types::ToolCall]) -> Result<()> {
        if let Some(app_handle) = &self.app_handle {
            let execution_start_event = serde_json::json!({
                "conversation_id": self.conversation_id,
                "message_id": self.message_id,
                "tool_calls": tool_calls.iter().map(|tc| {
                    serde_json::json!({
                        "id": tc.id,
                        "name": tc.name,
                        "status": "pending"
                    })
                }).collect::<Vec<_>>()
            });
            
            app_handle.emit("tool_execution_started", &execution_start_event)
                .map_err(|e| anyhow::anyhow!("Failed to emit tool execution start: {}", e))?;
        }
        Ok(())
    }

    /// Emit tool step start
    pub fn emit_tool_step_start(&self, tool_call_id: &str, tool_name: &str, step_index: usize, total_tools: usize) -> Result<()> {
        if let Some(app_handle) = &self.app_handle {
            let step_start_event = serde_json::json!({
                "conversation_id": self.conversation_id,
                "message_id": self.message_id,
                "tool_call_id": tool_call_id,
                "tool_name": tool_name,
                "step_index": step_index,
                "total_tools": total_tools,
                "status": "executing",
                "started_at": chrono::Utc::now().timestamp()
            });
            
            app_handle.emit("tool_step_started", &step_start_event)
                .map_err(|e| anyhow::anyhow!("Failed to emit tool step start: {}", e))?;
        }
        Ok(())
    }

    /// Emit tool step complete
    pub fn emit_tool_step_complete(
        &self,
        tool_call_id: &str,
        tool_name: &str,
        step_index: usize,
        total_tools: usize,
        result: &Result<Value, anyhow::Error>
    ) -> Result<()> {
        if let Some(app_handle) = &self.app_handle {
            let step_complete_event = serde_json::json!({
                "conversation_id": self.conversation_id,
                "message_id": self.message_id,
                "tool_call_id": tool_call_id,
                "tool_name": tool_name,
                "step_index": step_index,
                "total_tools": total_tools,
                "status": if result.is_ok() { "completed" } else { "failed" },
                "completed_at": chrono::Utc::now().timestamp(),
                "result": result.as_ref().ok(),
                "error": result.as_ref().err().map(|e| e.to_string()),
                "progress": ((step_index + 1) as f32 / total_tools as f32 * 100.0) as u32
            });
            
            app_handle.emit("tool_step_completed", &step_complete_event)
                .map_err(|e| anyhow::anyhow!("Failed to emit tool step complete: {}", e))?;
        }
        Ok(())
    }

    /// Emit tool execution complete
    pub fn emit_tool_execution_complete(&self, total_tools: usize) -> Result<()> {
        if let Some(app_handle) = &self.app_handle {
            let execution_complete_event = serde_json::json!({
                "conversation_id": self.conversation_id,
                "message_id": self.message_id,
                "total_tools": total_tools,
                "completed_tools": total_tools,
                "status": "all_completed"
            });
            
            app_handle.emit("tool_execution_completed", &execution_complete_event)
                .map_err(|e| anyhow::anyhow!("Failed to emit tool execution complete: {}", e))?;
        }
        Ok(())
    }
}

/// Stream processing helper for handling AI provider responses
pub struct StreamProcessor {
    provider_name: String,
}

impl StreamProcessor {
    pub fn new(provider_name: String) -> Self {
        Self { provider_name }
    }

    /// Parse provider-specific stream chunks
    pub fn parse_chunk(&self, raw_chunk: &crate::ai_adapter::types::StreamChunk) -> Option<crate::ai_adapter::types::StreamChunk> {
        if raw_chunk.id == "raw" {
            match self.provider_name.as_str() {
                "modelscope" => {
                    use crate::ai_adapter::providers::modelscope::ModelScopeProvider;
                    match ModelScopeProvider::parse_stream_chunk(&raw_chunk.content, &raw_chunk.model) {
                        Ok(Some(chunk)) => Some(chunk),
                        Ok(None) => None, // Skip empty chunks
                        Err(e) => {
                            warn!("Failed to parse ModelScope chunk: {}", e);
                            None
                        }
                    }
                },
                "moonshot" => {
                    use crate::ai_adapter::providers::moonshot::MoonshotProvider;
                    match MoonshotProvider::parse_stream_chunk(&raw_chunk.content) {
                        Ok(Some(chunk)) => Some(chunk),
                        Ok(None) => None, // Skip empty chunks
                        Err(e) => {
                            warn!("Failed to parse Moonshot chunk: {}", e);
                            None
                        }
                    }
                },
                "openai" => {
                    use crate::ai_adapter::providers::openai::OpenAiProvider;
                    match OpenAiProvider::parse_stream_chunk(&raw_chunk.content) {
                        Ok(Some(chunk)) => Some(chunk),
                        Ok(None) => None, // Skip empty chunks
                        Err(e) => {
                            warn!("Failed to parse OpenAI chunk: {}", e);
                            None
                        }
                    }
                },
                "openrouter" => {
                    use crate::ai_adapter::providers::openrouter::OpenRouterProvider;
                    match OpenRouterProvider::parse_stream_chunk(&raw_chunk.content) {
                        Ok(Some(chunk)) => Some(chunk),
                        Ok(None) => None, // Skip empty chunks
                        Err(e) => {
                            warn!("Failed to parse OpenRouter chunk: {}", e);
                            None
                        }
                    }
                },
                "deepseek" => {
                    use crate::ai_adapter::providers::deepseek::DeepSeekProvider;
                    match DeepSeekProvider::parse_stream_chunk(&raw_chunk.content) {
                        Ok(Some(chunk)) => Some(chunk),
                        Ok(None) => None, // Skip empty chunks
                        Err(e) => {
                            warn!("Failed to parse DeepSeek chunk: {}", e);
                            None
                        }
                    }
                },
                _ => {
                    warn!("Unknown provider '{}', using raw chunk", self.provider_name);
                    Some(raw_chunk.clone())
                }
            }
        } else {
            // Already parsed chunk or finish signal
            Some(raw_chunk.clone())
        }
    }

    /// Validate stream completion
    pub fn validate_completion(&self, chunk_count: u32, content_length: usize, finish_reason: &Option<String>) -> Result<()> {
        if chunk_count == 0 {
            return Err(anyhow::anyhow!(
                "No response received from AI provider '{}'. Please check your API configuration.", 
                self.provider_name
            ));
        }

        if content_length == 0 && finish_reason.is_none() {
            return Err(anyhow::anyhow!(
                "AI provider '{}' returned empty response. This may indicate an API key issue or the model doesn't support your request.", 
                self.provider_name
            ));
        }

        Ok(())
    }
}