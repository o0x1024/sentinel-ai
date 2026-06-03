//! AI Service trait for abstracting AI service operations

use async_trait::async_trait;
use anyhow::Result;
use crate::chunk_type::ChunkType;

/// AI Service trait - abstracts AI service operations
#[async_trait]
pub trait AiService: Send + Sync {
    /// Send a message stream (for LLM calls)
    async fn send_message_stream(
        &self,
        system_prompt: Option<&str>,
        user_message: Option<&str>,
        conversation_id: Option<&str>,
        context: Option<&str>,
        stream: bool,
        save_to_db: bool,
        chunk_type: Option<ChunkType>,
    ) -> Result<String>;
}

