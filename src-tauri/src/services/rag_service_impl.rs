//! RagQueryService trait implementation wrapping existing RAG system

use sentinel_engines::RagQueryService;
use anyhow::Result;
use async_trait::async_trait;
use std::sync::Arc;
use crate::commands::rag_commands::get_global_rag_service;
use sentinel_rag::models::AssistantRagRequest;

/// RAG service implementation wrapping the global RAG service
#[derive(Clone)]
pub struct RagServiceImpl;

impl RagServiceImpl {
    pub fn new() -> Self {
        Self
    }
}

#[async_trait]
impl RagQueryService for RagServiceImpl {
    async fn query_for_assistant(
        &self,
        request: &AssistantRagRequest,
    ) -> anyhow::Result<(String, Vec<String>)> {
        // Get the global RAG service
        let rag_service = get_global_rag_service()
            .await
            .map_err(|e| anyhow::anyhow!("Failed to get RAG service: {}", e))?;
        
        // Execute query - pass reference not clone
        let (context, citations) = rag_service.query_for_assistant(request).await?;
        
        // Convert Vec<Citation> to Vec<String> (citation text or references)
        let citation_strings: Vec<String> = citations.iter()
            .map(|c| format!("[{}]: {}", c.source_id, c.content_preview))
            .collect();
        
        Ok((context, citation_strings))
    }
}
