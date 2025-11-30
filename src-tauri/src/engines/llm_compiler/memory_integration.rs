//! LLM Compiler Memory Integration
//!
//! Provides memory-enhanced capabilities for the LLM Compiler engine:
//! - Execution trajectory storage and retrieval
//! - Failure pattern recognition for replanning
//! - Context summarization for long executions
//! - Tool call caching with intelligent TTL

use crate::engines::memory::memory::{
    ExecutionExperience, LearningUpdate, Memory, MemoryQuery, PlanTemplate, SimilaritySearchResult,
};
use crate::engines::memory::IntelligentMemory;
use crate::engines::types::StepExecutionStatus;
use anyhow::Result;
use chrono::Utc;
use serde_json::{json, Value};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;
use tracing::{debug, info, warn};
use uuid::Uuid;

/// LLM Compiler Memory Integration configuration
#[derive(Debug, Clone)]
pub struct LlmCompilerMemoryConfig {
    /// Enable execution trajectory storage
    pub enable_trajectory_storage: bool,
    /// Enable failure pattern learning
    pub enable_failure_learning: bool,
    /// Enable context summarization
    pub enable_context_summarization: bool,
    /// Summarization threshold (number of tasks)
    pub summarization_threshold: usize,
    /// Maximum context tokens before summarization
    pub max_context_tokens: usize,
    /// Similarity threshold for experience retrieval
    pub similarity_threshold: f64,
    /// Maximum similar experiences to retrieve
    pub max_similar_experiences: usize,
}

impl Default for LlmCompilerMemoryConfig {
    fn default() -> Self {
        Self {
            enable_trajectory_storage: true,
            enable_failure_learning: true,
            enable_context_summarization: true,
            summarization_threshold: 10,
            max_context_tokens: 8000,
            similarity_threshold: 0.6,
            max_similar_experiences: 5,
        }
    }
}

/// LLM Compiler Memory Integration
pub struct LlmCompilerMemoryIntegration {
    memory: Arc<RwLock<IntelligentMemory>>,
    config: LlmCompilerMemoryConfig,
    /// Current execution trajectory
    current_trajectory: RwLock<Vec<TaskTrajectoryEntry>>,
    /// Summarized context for long executions
    summarized_context: RwLock<Option<String>>,
}

/// Task trajectory entry for execution history
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct TaskTrajectoryEntry {
    pub task_id: String,
    pub task_name: String,
    pub tool_name: String,
    pub inputs: HashMap<String, Value>,
    pub outputs: Option<HashMap<String, Value>>,
    pub status: String,
    pub error: Option<String>,
    pub duration_ms: u64,
    pub timestamp: i64,
}

impl LlmCompilerMemoryIntegration {
    /// Create a new memory integration instance
    pub fn new(memory: Arc<RwLock<IntelligentMemory>>) -> Self {
        Self::with_config(memory, LlmCompilerMemoryConfig::default())
    }

    /// Create with custom configuration
    pub fn with_config(memory: Arc<RwLock<IntelligentMemory>>, config: LlmCompilerMemoryConfig) -> Self {
        Self {
            memory,
            config,
            current_trajectory: RwLock::new(Vec::new()),
            summarized_context: RwLock::new(None),
        }
    }

    /// Record a task execution in the trajectory
    pub async fn record_task_execution(
        &self,
        task_id: &str,
        task_name: &str,
        tool_name: &str,
        inputs: &HashMap<String, Value>,
        outputs: Option<&HashMap<String, Value>>,
        status: &str,
        error: Option<&str>,
        duration_ms: u64,
    ) {
        if !self.config.enable_trajectory_storage {
            return;
        }

        let entry = TaskTrajectoryEntry {
            task_id: task_id.to_string(),
            task_name: task_name.to_string(),
            tool_name: tool_name.to_string(),
            inputs: inputs.clone(),
            outputs: outputs.cloned(),
            status: status.to_string(),
            error: error.map(|s| s.to_string()),
            duration_ms,
            timestamp: Utc::now().timestamp(),
        };

        let mut trajectory = self.current_trajectory.write().await;
        trajectory.push(entry);

        debug!(
            "Recorded task execution: {} ({}), status: {}",
            task_name, tool_name, status
        );

        // Check if we need to summarize
        if trajectory.len() >= self.config.summarization_threshold {
            drop(trajectory);
            self.maybe_summarize_context().await;
        }
    }

    /// Store the complete execution trajectory when workflow finishes
    pub async fn store_execution_trajectory(
        &self,
        task_description: &str,
        success: bool,
        final_response: Option<&str>,
    ) -> Result<()> {
        if !self.config.enable_trajectory_storage {
            return Ok(());
        }

        let trajectory = self.current_trajectory.read().await;
        if trajectory.is_empty() {
            return Ok(());
        }

        let steps: Vec<Value> = trajectory
            .iter()
            .map(|entry| {
                json!({
                    "task_id": entry.task_id,
                    "task_name": entry.task_name,
                    "tool_name": entry.tool_name,
                    "inputs": entry.inputs,
                    "outputs": entry.outputs,
                    "status": entry.status,
                    "error": entry.error,
                    "duration_ms": entry.duration_ms,
                })
            })
            .collect();

        let error_info = if !success {
            trajectory
                .iter()
                .filter(|e| e.status == "Failed")
                .last()
                .map(|e| {
                    json!({
                        "task_id": e.task_id,
                        "tool_name": e.tool_name,
                        "error": e.error,
                    })
                })
        } else {
            None
        };

        let mut memory = self.memory.write().await;
        memory.store_execution_trajectory(
            task_description.to_string(),
            steps,
            success,
            error_info,
        )?;

        info!(
            "Stored execution trajectory: {} tasks, success: {}",
            trajectory.len(),
            success
        );

        Ok(())
    }

    /// Retrieve similar failure trajectories for replanning guidance
    pub async fn retrieve_failure_patterns(
        &self,
        task_description: &str,
        error_pattern: &str,
    ) -> Result<Vec<SimilaritySearchResult<ExecutionExperience>>> {
        if !self.config.enable_failure_learning {
            return Ok(Vec::new());
        }

        let memory = self.memory.read().await;
        memory.retrieve_failure_trajectories(task_description, error_pattern)
    }

    /// Retrieve similar successful execution plans
    pub async fn retrieve_similar_plans(
        &self,
        task_description: &str,
    ) -> Result<Vec<SimilaritySearchResult<PlanTemplate>>> {
        let memory = self.memory.read().await;
        memory.retrieve_few_shot_plans(task_description, self.config.max_similar_experiences)
    }

    /// Retrieve similar execution experiences
    pub async fn retrieve_similar_experiences(
        &self,
        task_type: &str,
        target_description: &str,
        environment_context: &str,
    ) -> Result<Vec<SimilaritySearchResult<ExecutionExperience>>> {
        let memory = self.memory.read().await;
        let query = MemoryQuery {
            query_type: crate::engines::memory::memory::QueryType::SuccessfulPatterns,
            task_type: Some(task_type.to_string()),
            target_description: Some(target_description.to_string()),
            environment_context: Some(environment_context.to_string()),
            tool_names: None,
            error_patterns: None,
            similarity_threshold: self.config.similarity_threshold,
            max_results: self.config.max_similar_experiences,
            include_metadata: false,
        };
        memory.retrieve_similar_experiences(&query)
    }

    /// Check tool call cache
    pub async fn check_tool_cache(
        &self,
        tool_name: &str,
        tool_args: &Value,
    ) -> Result<Option<Value>> {
        let memory = self.memory.read().await;
        memory.check_tool_call_cache(tool_name, tool_args)
    }

    /// Cache tool call result
    pub async fn cache_tool_result(
        &self,
        tool_name: String,
        tool_args: Value,
        result: Value,
        execution_time_ms: u64,
    ) -> Result<()> {
        let mut memory = self.memory.write().await;
        memory.cache_tool_call_result(tool_name, tool_args, result, execution_time_ms)
    }

    /// Learn from execution result
    pub async fn learn_from_execution(
        &self,
        session_id: &str,
        task_description: &str,
        success: bool,
        performance_metrics: HashMap<String, f64>,
    ) -> Result<()> {
        if !self.config.enable_failure_learning {
            return Ok(());
        }

        let trajectory = self.current_trajectory.read().await;
        let context_info = json!({
            "task_description": task_description,
            "total_tasks": trajectory.len(),
            "successful_tasks": trajectory.iter().filter(|t| t.status == "Completed").count(),
            "failed_tasks": trajectory.iter().filter(|t| t.status == "Failed").count(),
        });

        let total_duration_ms: u64 = trajectory.iter().map(|t| t.duration_ms).sum();

        // Create ExecutionMetrics with correct fields
        let exec_metrics = crate::engines::types::ExecutionMetrics {
            execution_time_ms: total_duration_ms,
            memory_usage_bytes: 0,
            cpu_usage_percent: 0.0,
            network_io_bytes: 0,
            disk_io_bytes: 0,
            custom_metrics: HashMap::new(),
        };

        // Create StepExecutionResult with correct fields
        let execution_result = crate::engines::types::StepExecutionResult {
            step_id: session_id.to_string(),
            status: if success {
                StepExecutionStatus::Completed
            } else {
                StepExecutionStatus::Failed
            },
            started_at: std::time::SystemTime::now(),
            completed_at: Some(std::time::SystemTime::now()),
            result_data: None,
            error: None,
            retry_count: 0,
            logs: Vec::new(),
            metrics: exec_metrics.clone(),
        };

        let update = LearningUpdate {
            session_id: session_id.to_string(),
            execution_result,
            user_feedback: None,
            performance_metrics: exec_metrics,
            context_info,
        };

        let mut memory = self.memory.write().await;
        memory.learn_from_execution(update)?;

        info!(
            "Learned from execution: session={}, success={}",
            session_id, success
        );
        Ok(())
    }

    /// Get tool effectiveness from memory
    pub async fn get_tool_effectiveness(
        &self,
        tool_name: &str,
        target_type: Option<&str>,
        environment: Option<&str>,
    ) -> Result<f64> {
        let memory = self.memory.read().await;
        memory.get_tool_effectiveness(tool_name, target_type, environment)
    }

    /// Get environment-specific recommendations
    pub async fn get_recommendations(
        &self,
        environment: &str,
        task_type: &str,
    ) -> Result<Vec<String>> {
        let memory = self.memory.read().await;
        memory.get_environment_specific_recommendations(environment, task_type)
    }

    /// Maybe summarize context if trajectory is too long
    async fn maybe_summarize_context(&self) {
        if !self.config.enable_context_summarization {
            return;
        }

        let trajectory = self.current_trajectory.read().await;
        if trajectory.len() < self.config.summarization_threshold {
            return;
        }

        // Generate summary of completed tasks
        let summary = self.generate_trajectory_summary(&trajectory);
        
        let mut summarized = self.summarized_context.write().await;
        *summarized = Some(summary);

        debug!(
            "Context summarized: {} tasks -> summary generated",
            trajectory.len()
        );
    }

    /// Generate a summary of the execution trajectory
    fn generate_trajectory_summary(&self, trajectory: &[TaskTrajectoryEntry]) -> String {
        let completed: Vec<_> = trajectory.iter().filter(|t| t.status == "Completed").collect();
        let failed: Vec<_> = trajectory.iter().filter(|t| t.status == "Failed").collect();

        let mut summary = String::new();
        summary.push_str(&format!(
            "Execution Progress: {} completed, {} failed\n",
            completed.len(),
            failed.len()
        ));

        // Summarize completed tasks
        if !completed.is_empty() {
            summary.push_str("\nCompleted tasks:\n");
            for task in completed.iter().take(5) {
                summary.push_str(&format!(
                    "- {} ({}): {:?}\n",
                    task.task_name,
                    task.tool_name,
                    task.outputs.as_ref().map(|o| o.keys().collect::<Vec<_>>())
                ));
            }
            if completed.len() > 5 {
                summary.push_str(&format!("... and {} more\n", completed.len() - 5));
            }
        }

        // Summarize failed tasks
        if !failed.is_empty() {
            summary.push_str("\nFailed tasks:\n");
            for task in &failed {
                summary.push_str(&format!(
                    "- {} ({}): {}\n",
                    task.task_name,
                    task.tool_name,
                    task.error.as_deref().unwrap_or("Unknown error")
                ));
            }
        }

        summary
    }

    /// Get current summarized context
    pub async fn get_summarized_context(&self) -> Option<String> {
        self.summarized_context.read().await.clone()
    }

    /// Get current trajectory length
    pub async fn get_trajectory_length(&self) -> usize {
        self.current_trajectory.read().await.len()
    }

    /// Clear current trajectory (call when starting new execution)
    pub async fn clear_trajectory(&self) {
        let mut trajectory = self.current_trajectory.write().await;
        trajectory.clear();
        let mut summarized = self.summarized_context.write().await;
        *summarized = None;
    }

    /// Build context augmentation for planner/joiner prompts
    pub async fn build_context_augmentation(&self, task_description: &str) -> String {
        let mut augmentation = String::new();

        // Add summarized context if available
        if let Some(summary) = self.get_summarized_context().await {
            augmentation.push_str("\n[EXECUTION PROGRESS]\n");
            augmentation.push_str(&summary);
        }

        // Try to retrieve similar experiences
        if let Ok(experiences) = self
            .retrieve_similar_experiences("security_scan", task_description, "")
            .await
        {
            if !experiences.is_empty() {
                augmentation.push_str("\n[SIMILAR PAST EXECUTIONS]\n");
                for exp in experiences.iter().take(3) {
                    augmentation.push_str(&format!(
                        "- Task: {}, Success: {}, Confidence: {:.2}\n",
                        exp.item.task_type,
                        exp.item.confidence_score > 0.7,
                        exp.item.confidence_score
                    ));
                }
            }
        }

        // Try to get recommendations
        if let Ok(recommendations) = self.get_recommendations("default", "security_scan").await {
            if !recommendations.is_empty() && recommendations[0] != "No specific recommendations available for this environment" {
                augmentation.push_str("\n[RECOMMENDATIONS]\n");
                for rec in recommendations.iter().take(3) {
                    augmentation.push_str(&format!("- {}\n", rec));
                }
            }
        }

        augmentation
    }
}

/// Context summarizer for LLM Compiler
pub struct LlmCompilerContextSummarizer {
    max_tokens: usize,
}

impl LlmCompilerContextSummarizer {
    pub fn new(max_tokens: usize) -> Self {
        Self { max_tokens }
    }

    /// Estimate token count (simple heuristic)
    pub fn estimate_tokens(&self, text: &str) -> usize {
        // Rough estimation: 1 token ≈ 4 characters
        text.len() / 4
    }

    /// Check if context needs summarization
    pub fn needs_summarization(&self, context: &str) -> bool {
        self.estimate_tokens(context) > self.max_tokens
    }

    /// Truncate context to fit within token limit
    pub fn truncate_context(&self, context: &str) -> String {
        let estimated_tokens = self.estimate_tokens(context);
        if estimated_tokens <= self.max_tokens {
            return context.to_string();
        }

        // Keep the most recent portion
        let target_chars = self.max_tokens * 4;
        if context.len() > target_chars {
            let truncated = &context[context.len() - target_chars..];
            format!("[...truncated...]\n{}", truncated)
        } else {
            context.to_string()
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_memory_integration_creation() {
        let memory = Arc::new(RwLock::new(IntelligentMemory::new()));
        let integration = LlmCompilerMemoryIntegration::new(memory);
        
        assert_eq!(integration.get_trajectory_length().await, 0);
    }

    #[tokio::test]
    async fn test_trajectory_recording() {
        let memory = Arc::new(RwLock::new(IntelligentMemory::new()));
        let integration = LlmCompilerMemoryIntegration::new(memory);

        let inputs = HashMap::new();
        integration
            .record_task_execution(
                "task_1",
                "Test Task",
                "test_tool",
                &inputs,
                None,
                "Completed",
                None,
                100,
            )
            .await;

        assert_eq!(integration.get_trajectory_length().await, 1);
    }

    #[test]
    fn test_context_summarizer() {
        let summarizer = LlmCompilerContextSummarizer::new(100);
        
        let short_text = "Hello world";
        assert!(!summarizer.needs_summarization(short_text));

        let long_text = "a".repeat(1000);
        assert!(summarizer.needs_summarization(&long_text));
    }
}

