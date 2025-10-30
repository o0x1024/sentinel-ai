//! Unified tool manager (library)

use crate::unified_types::*;
use anyhow::{anyhow, Result};
use chrono::Utc;
use futures;
use serde_json::Value;
use std::collections::HashMap;
use std::sync::Arc;
use std::time::Instant;
use tokio::sync::RwLock;
use tokio::time::timeout;
use tracing::{debug, error, info, warn};
use uuid::Uuid;

#[derive(Debug)]
pub struct UnifiedToolManager {
    providers: HashMap<String, Box<dyn ToolProvider>>,
    tool_registry: Arc<RwLock<HashMap<String, Arc<dyn UnifiedTool>>>>,
    execution_history: Arc<RwLock<Vec<ToolExecutionRecord>>>,
    config: ToolManagerConfig,
    execution_counter: Arc<RwLock<usize>>,
}

impl UnifiedToolManager {
    pub fn new(config: ToolManagerConfig) -> Self {
        Self {
            providers: HashMap::new(),
            tool_registry: Arc::new(RwLock::new(HashMap::new())),
            execution_history: Arc::new(RwLock::new(Vec::new())),
            config,
            execution_counter: Arc::new(RwLock::new(0)),
        }
    }

    pub async fn register_provider(&mut self, provider: Box<dyn ToolProvider>) -> Result<()> {
        let provider_name = provider.name().to_string();
        debug!("Registering tool provider: {}", provider_name);

        if !provider.is_available().await {
            warn!("Tool provider {} is not available", provider_name);
            return Err(anyhow!("Tool provider {} is not available", provider_name));
        }

        let tools = provider.get_tools().await?;
        let mut registry = self.tool_registry.write().await;
        for tool in tools {
            let tool_name = tool.name().to_string();
            debug!("Registering tool: {} from provider: {}", tool_name, provider_name);
            registry.insert(tool_name, tool);
        }

        self.providers.insert(provider_name, provider);
        Ok(())
    }

    pub async fn refresh_all_providers(&mut self) -> Result<()> {
        info!("Refreshing all tool providers");
        for (provider_name, provider) in &self.providers {
            info!("Refreshing provider: {}", provider_name);
            if let Err(e) = provider.refresh().await {
                error!("Failed to refresh provider {}: {}", provider_name, e);
                continue;
            }

            match provider.get_tools().await {
                Ok(tools) => {
                    let mut registry = self.tool_registry.write().await;
                    for tool in tools {
                        let tool_name = tool.name().to_string();
                        registry.insert(tool_name, tool);
                    }
                }
                Err(e) => error!("Failed to get tools from provider {}: {}", provider_name, e),
            }
        }
        Ok(())
    }

    pub async fn call_tool(&self, tool_name: &str, params: ToolExecutionParams) -> Result<ToolExecutionResult> {
        let start_time = Instant::now();
        let execution_id = params.execution_id.unwrap_or_else(Uuid::new_v4);
        info!("Executing tool: {} with execution_id: {}", tool_name, execution_id);

        {
            let mut counter = self.execution_counter.write().await;
            *counter += 1;
            if *counter > self.config.max_concurrent_executions {
                *counter -= 1;
                return Err(anyhow!("Maximum concurrent executions ({}) exceeded", self.config.max_concurrent_executions));
            }
        }

        let tool = {
            let registry = self.tool_registry.read().await;
            registry.get(tool_name).cloned()
        };

        let tool = match tool { Some(t) => t, None => { self.decrement_counter().await; return Err(anyhow!("Tool '{}' not found", tool_name)); } };

        if let Err(e) = tool.validate_params(&params) { self.decrement_counter().await; return Err(anyhow!("Parameter validation failed for tool '{}': {}", tool_name, e)); }
        if !tool.is_available().await { self.decrement_counter().await; return Err(anyhow!("Tool '{}' is not available", tool_name)); }

        let execution_timeout = params.timeout.unwrap_or(self.config.default_timeout);
        let tool_params = ToolExecutionParams { inputs: params.inputs.clone(), context: params.context.clone(), timeout: params.timeout, execution_id: Some(execution_id) };

        let result = match timeout(execution_timeout, tool.execute(tool_params)).await {
            Ok(Ok(mut result)) => { result.execution_id = execution_id; result.execution_time_ms = start_time.elapsed().as_millis() as u64; result.started_at = Utc::now(); result.completed_at = Some(Utc::now()); Ok(result) }
            Ok(Err(e)) => { error!("Tool execution failed: {}", e); Err(e) }
            Err(_) => { error!("Tool execution timed out after {:?}", execution_timeout); Err(anyhow!("Tool execution timed out after {:?}", execution_timeout)) }
        };

        self.decrement_counter().await;

        if self.config.log_executions {
            let record = ToolExecutionRecord { execution_id, tool_name: tool_name.to_string(), params, result: result.as_ref().ok().cloned(), created_at: Utc::now() };
            let mut history = self.execution_history.write().await;
            history.push(record);
            if history.len() > 1000 { history.drain(0..100); }
        }

        result
    }

    pub async fn list_tools(&self) -> Vec<ToolInfo> {
        let registry = self.tool_registry.read().await;
        let mut tools = Vec::new();
        for tool in registry.values() {
            let tool_info = ToolInfo { id: tool.name().to_string(), name: tool.name().to_string(), description: tool.description().to_string(), version: tool.metadata().version.clone(), category: tool.category(), parameters: tool.parameters().clone(), metadata: tool.metadata().clone(), available: tool.is_available().await, installed: tool.is_installed().await, source: ToolSource::Builtin };
            tools.push(tool_info);
        }
        tools
    }

    pub async fn search_tools(&self, query: ToolSearchQuery) -> ToolSearchResult {
        let all_tools = self.list_tools().await;
        let query_lower = query.query.to_lowercase();
        let filtered_tools: Vec<ToolInfo> = all_tools.into_iter().filter(|tool| {
            let text_match = tool.name.to_lowercase().contains(&query_lower) || tool.description.to_lowercase().contains(&query_lower) || tool.metadata.tags.iter().any(|tag| tag.to_lowercase().contains(&query_lower));
            let category_match = query.category.as_ref().map(|cat| std::mem::discriminant(cat) == std::mem::discriminant(&tool.category)).unwrap_or(true);
            let tag_match = query.tags.is_empty() || query.tags.iter().any(|tag| tool.metadata.tags.iter().any(|tool_tag| tool_tag.to_lowercase().contains(&tag.to_lowercase())));
            let available_filter = !query.available_only || tool.available;
            let installed_filter = !query.installed_only || tool.installed;
            text_match && category_match && tag_match && available_filter && installed_filter
        }).collect();

        ToolSearchResult { total_count: filtered_tools.len(), tools: filtered_tools, query }
    }

    pub async fn execute_batch(&self, request: BatchExecutionRequest) -> Result<BatchExecutionResult> {
        let batch_id = Uuid::new_v4();
        let start_time = Instant::now();
        let started_at = Utc::now();
        info!("Starting batch execution: {} with {} tools", batch_id, request.requests.len());
        let mut results = Vec::new();
        let mut success_count = 0;
        let mut failure_count = 0;
        match request.mode {
            BatchExecutionMode::Parallel => {
                let mut futures_vec = Vec::new();
                for req in &request.requests { futures_vec.push(self.call_tool(&req.tool_name, req.params.clone())); }
                let batch_results = futures::future::join_all(futures_vec).await;
                for result in batch_results { match result { Ok(exec_result) => { if exec_result.success { success_count += 1; } else { failure_count += 1; } results.push(exec_result); } Err(e) => { error!("Batch execution error: {}", e); failure_count += 1; if request.stop_on_error { break; } } } }
            }
            BatchExecutionMode::Sequential => {
                for req in request.requests { match self.call_tool(&req.tool_name, req.params).await { Ok(exec_result) => { if exec_result.success { success_count += 1; } else { failure_count += 1; } results.push(exec_result); } Err(e) => { error!("Sequential execution error: {}", e); failure_count += 1; if request.stop_on_error { break; } } } }
            }
            BatchExecutionMode::Pipeline => {
                let mut previous_output: Option<Value> = None;
                for mut req in request.requests {
                    if let Some(ref output) = previous_output { req.params.inputs.insert("pipeline_input".to_string(), output.clone()); }
                    match self.call_tool(&req.tool_name, req.params).await { Ok(exec_result) => { if exec_result.success { success_count += 1; previous_output = Some(exec_result.output.clone()); } else { failure_count += 1; if request.stop_on_error { break; } } results.push(exec_result); } Err(e) => { error!("Pipeline execution error: {}", e); failure_count += 1; if request.stop_on_error { break; } } }
                }
            }
        }
        let total_execution_time_ms = start_time.elapsed().as_millis() as f64;
        Ok(BatchExecutionResult { batch_id, results, success_count, failure_count, total_execution_time_ms, started_at, completed_at: Some(Utc::now()) })
    }

    pub async fn get_execution_history(&self, limit: Option<usize>) -> Vec<ToolExecutionRecord> {
        let history = self.execution_history.read().await;
        let limit = limit.unwrap_or(100);
        if history.len() <= limit { history.clone() } else { history[history.len() - limit..].to_vec() }
    }

    pub async fn get_tool_statistics(&self) -> HashMap<String, ToolStatistics> {
        let history = self.execution_history.read().await;
        let mut stats: HashMap<String, ToolStatistics> = HashMap::new();
        for record in history.iter() {
            let tool_stats = stats.entry(record.tool_name.clone()).or_insert_with(|| ToolStatistics { tool_name: record.tool_name.clone(), total_executions: 0, successful_executions: 0, failed_executions: 0, average_execution_time_ms: 0, total_execution_time_ms: 0 });
            tool_stats.total_executions += 1;
            if let Some(ref result) = record.result { if result.success { tool_stats.successful_executions += 1; } else { tool_stats.failed_executions += 1; } tool_stats.total_execution_time_ms += result.execution_time_ms; } else { tool_stats.failed_executions += 1; }
        }
        for stat in stats.values_mut() { if stat.total_executions > 0 { stat.average_execution_time_ms = stat.total_execution_time_ms as u64 / stat.total_executions as u64; } }
        stats
    }

    pub async fn clear_execution_history(&self) {
        let mut history = self.execution_history.write().await;
        history.clear();
        info!("Execution history cleared");
    }

    async fn decrement_counter(&self) {
        let mut counter = self.execution_counter.write().await;
        if *counter > 0 { *counter -= 1; }
    }
}


