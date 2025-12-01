//! ReAct Memory 集成模块
//!
//! 将智能记忆系统集成到 ReAct 循环中，支持：
//! - 检索相似推理链（Few-shot 示例）
//! - 存储执行轨迹（经验学习）
//! - 工具调用缓存（避免重复调用）

use super::types::*;
use crate::engines::memory::memory::{
    ExecutionExperience, Memory, MemoryQuery, PlanTemplate, QueryType, SimilaritySearchResult,
};
use crate::engines::memory::IntelligentMemory;
use anyhow::Result;
use chrono::Utc;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;

/// ReAct Memory 集成器
pub struct ReactMemoryIntegration {
    memory: Arc<RwLock<IntelligentMemory>>,
    config: ReactMemoryConfig,
    /// 工具调用缓存（内存级，用于当前会话）
    tool_cache: Arc<RwLock<HashMap<String, CachedToolResult>>>,
}

/// ReAct Memory 配置
#[derive(Debug, Clone)]
pub struct ReactMemoryConfig {
    /// 检索推理链的最大数量
    pub max_reasoning_chains: usize,
    /// 相似度阈值
    pub similarity_threshold: f64,
    /// 是否启用工具缓存
    pub enable_tool_cache: bool,
    /// 工具缓存过期时间（秒）
    pub tool_cache_ttl_seconds: u64,
    /// Context 摘要阈值（超过此步数时进行摘要）
    pub summarization_threshold: usize,
}

impl Default for ReactMemoryConfig {
    fn default() -> Self {
        Self {
            max_reasoning_chains: 3,
            similarity_threshold: 0.6,
            enable_tool_cache: true,
            tool_cache_ttl_seconds: 300, // 5分钟
            summarization_threshold: 8,
        }
    }
}

/// 缓存的工具调用结果
#[derive(Debug, Clone, Serialize, Deserialize)]
struct CachedToolResult {
    result: serde_json::Value,
    cached_at: i64,
    execution_time_ms: u64,
}

/// 检索到的推理链示例
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReasoningChainExample {
    /// 任务描述
    pub task: String,
    /// 推理步骤摘要
    pub steps_summary: String,
    /// 最终答案
    pub final_answer: Option<String>,
    /// 成功率
    pub success_rate: f64,
    /// 相似度分数
    pub similarity_score: f64,
}

impl ReactMemoryIntegration {
    /// 创建新的 Memory 集成器
    pub fn new(memory: Arc<RwLock<IntelligentMemory>>) -> Self {
        Self::with_config(memory, ReactMemoryConfig::default())
    }

    /// 使用自定义配置创建
    pub fn with_config(memory: Arc<RwLock<IntelligentMemory>>, config: ReactMemoryConfig) -> Self {
        Self {
            memory,
            config,
            tool_cache: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    /// 思考前：检索相似推理链作为 Few-shot 示例
    ///
    /// 返回历史上成功处理过类似任务的推理链，可作为提示词中的示例
    pub async fn retrieve_reasoning_chains(
        &self,
        task_description: &str,
    ) -> Result<Vec<ReasoningChainExample>> {
        log::info!(
            "Retrieving reasoning chains for task: {}",
            &task_description[..task_description.len().min(100)]
        );

        let memory_guard = self.memory.read().await;

        // 使用 Memory trait 的 retrieve_reasoning_chains 方法
        let results = memory_guard.retrieve_reasoning_chains(
            task_description,
            self.config.max_reasoning_chains,
        )?;

        // 转换为 ReasoningChainExample
        let examples: Vec<ReasoningChainExample> = results
            .into_iter()
            .filter(|r| r.similarity_score >= self.config.similarity_threshold)
            .map(|result| {
                // 从 successful_steps 构建摘要
                let steps_summary = self.build_steps_summary(&result.item.successful_steps);

                ReasoningChainExample {
                    task: result.item.task_type.clone(),
                    steps_summary,
                    final_answer: result
                        .item
                        .performance_metrics
                        .as_ref()
                        .and_then(|m| m.get("final_answer"))
                        .and_then(|v| v.as_str())
                        .map(|s| s.to_string()),
                    success_rate: result.item.confidence_score,
                    similarity_score: result.similarity_score,
                }
            })
            .collect();

        log::info!("Found {} relevant reasoning chain examples", examples.len());
        Ok(examples)
    }

    /// 执行完成后：存储完整的 ReAct 轨迹
    pub async fn store_trace(&self, trace: &ReactTrace) -> Result<()> {
        log::info!(
            "Storing ReAct trace: {} (status: {:?}, iterations: {})",
            trace.trace_id,
            trace.status,
            trace.metrics.total_iterations
        );

        // 构建 successful_steps
        let successful_steps: Vec<serde_json::Value> = trace
            .steps
            .iter()
            .filter_map(|step| match &step.step_type {
                ReactStepType::Action { tool_call } => Some(serde_json::json!({
                    "step_id": step.id,
                    "tool_name": tool_call.tool,
                    "parameters": tool_call.args,
                    "duration_ms": step.duration_ms,
                })),
                ReactStepType::Observation {
                    tool_name,
                    result,
                    success,
                } => {
                    if *success {
                        Some(serde_json::json!({
                            "step_id": step.id,
                            "tool_name": tool_name,
                            "result_preview": self.truncate_result(result),
                            "success": true,
                        }))
                    } else {
                        None
                    }
                }
                _ => None,
            })
            .collect();

        // 构建 failure_info
        let failure_info = if trace.status == ReactStatus::Failed
            || trace.status == ReactStatus::MaxIterationsReached
        {
            let errors: Vec<String> = trace
                .steps
                .iter()
                .filter_map(|step| step.error.clone())
                .collect();

            Some(serde_json::json!({
                "status": format!("{:?}", trace.status),
                "errors": errors,
                "last_step": trace.steps.last().map(|s| &s.id),
            }))
        } else {
            None
        };

        // 提取最终答案
        let final_answer = trace.steps.iter().find_map(|step| {
            if let ReactStepType::Final { answer, .. } = &step.step_type {
                Some(answer.clone())
            } else {
                None
            }
        });

        // 构建 ExecutionExperience
        let experience = ExecutionExperience {
            id: trace.trace_id.clone(),
            task_type: "react_reasoning".to_string(),
            target_description: trace.task.clone(),
            target_hash: format!("{:x}", md5::compute(&trace.task)),
            target_properties: None,
            environment_context: "react".to_string(),
            environment_hash: "react".to_string(),
            environment_properties: Some(serde_json::json!({
                "architecture": "ReAct",
                "max_iterations": trace.metrics.total_iterations,
            })),
            successful_steps,
            failure_info,
            performance_metrics: Some(serde_json::json!({
                "total_iterations": trace.metrics.total_iterations,
                "tool_calls_count": trace.metrics.tool_calls_count,
                "successful_tool_calls": trace.metrics.successful_tool_calls,
                "failed_tool_calls": trace.metrics.failed_tool_calls,
                "total_duration_ms": trace.metrics.total_duration_ms,
                "final_answer": final_answer,
            })),
            confidence_score: self.calculate_confidence_score(trace),
            usage_count: 1,
            last_used_at: Some(Utc::now().timestamp()),
            created_at: Utc::now().timestamp(),
            updated_at: Utc::now().timestamp(),
        };

        // 存储到 Memory
        let mut memory_guard = self.memory.write().await;
        memory_guard.store_experience(experience)?;

        log::info!(
            "Successfully stored ReAct trace {} to memory",
            trace.trace_id
        );
        Ok(())
    }

    /// 工具调用前：检查缓存
    pub async fn check_tool_cache(
        &self,
        tool_name: &str,
        args: &serde_json::Value,
    ) -> Result<Option<serde_json::Value>> {
        if !self.config.enable_tool_cache {
            return Ok(None);
        }

        let cache_key = self.build_cache_key(tool_name, args);
        let cache = self.tool_cache.read().await;

        if let Some(cached) = cache.get(&cache_key) {
            // 检查是否过期
            let now = Utc::now().timestamp();
            if (now - cached.cached_at) < self.config.tool_cache_ttl_seconds as i64 {
                log::debug!("Tool cache hit for {}({})", tool_name, cache_key);
                return Ok(Some(cached.result.clone()));
            }
        }

        Ok(None)
    }

    /// 工具调用后：更新缓存
    pub async fn cache_tool_result(
        &self,
        tool_name: &str,
        args: &serde_json::Value,
        result: &serde_json::Value,
        execution_time_ms: u64,
    ) -> Result<()> {
        if !self.config.enable_tool_cache {
            return Ok(());
        }

        // 只缓存成功的结果
        let is_error = result.get("error").is_some()
            || result.get("success").map(|v| v == false).unwrap_or(false);
        if is_error {
            return Ok(());
        }

        let cache_key = self.build_cache_key(tool_name, args);
        let cached = CachedToolResult {
            result: result.clone(),
            cached_at: Utc::now().timestamp(),
            execution_time_ms,
        };

        let mut cache = self.tool_cache.write().await;
        cache.insert(cache_key, cached);

        // 也存储到持久化 Memory（用于跨会话缓存）
        let mut memory_guard = self.memory.write().await;
        let _ = memory_guard.cache_tool_call_result(
            tool_name.to_string(),
            args.clone(),
            result.clone(),
            execution_time_ms,
        );

        log::debug!("Cached tool result for {}", tool_name);
        Ok(())
    }

    /// 检查持久化缓存（跨会话）
    pub async fn check_persistent_cache(
        &self,
        tool_name: &str,
        args: &serde_json::Value,
    ) -> Result<Option<serde_json::Value>> {
        let memory_guard = self.memory.read().await;
        memory_guard.check_tool_call_cache(tool_name, args)
    }

    /// 获取工具效果统计
    pub async fn get_tool_effectiveness(&self, tool_name: &str) -> Result<f64> {
        let memory_guard = self.memory.read().await;
        memory_guard.get_tool_effectiveness(tool_name, None, None)
    }

    /// 清理过期的工具缓存
    pub async fn cleanup_expired_cache(&self) {
        let now = Utc::now().timestamp();
        let mut cache = self.tool_cache.write().await;

        cache.retain(|_, v| (now - v.cached_at) < self.config.tool_cache_ttl_seconds as i64);

        log::debug!("Cleaned up expired tool cache, {} entries remaining", cache.len());
    }

    // ========== 私有辅助方法 ==========

    fn build_cache_key(&self, tool_name: &str, args: &serde_json::Value) -> String {
        let args_str = serde_json::to_string(args).unwrap_or_default();
        format!("{}:{:x}", tool_name, md5::compute(&args_str))
    }

    fn build_steps_summary(&self, steps: &[serde_json::Value]) -> String {
        let tool_calls: Vec<String> = steps
            .iter()
            .filter_map(|step| {
                step.get("tool_name")
                    .and_then(|v| v.as_str())
                    .map(|s| s.to_string())
            })
            .collect();

        if tool_calls.is_empty() {
            "No tool calls recorded".to_string()
        } else {
            format!("Tools used: {}", tool_calls.join(" -> "))
        }
    }

    fn truncate_result(&self, result: &serde_json::Value) -> serde_json::Value {
        let result_str = serde_json::to_string(result).unwrap_or_default();
        if result_str.len() > 500 {
            // 安全截断，确保不在 UTF-8 字符中间切断
            let mut end = 500;
            while end > 0 && !result_str.is_char_boundary(end) {
                end -= 1;
            }
            serde_json::json!({
                "truncated": true,
                "preview": &result_str[..end],
                "original_length": result_str.len()
            })
        } else {
            result.clone()
        }
    }

    fn calculate_confidence_score(&self, trace: &ReactTrace) -> f64 {
        match trace.status {
            ReactStatus::Completed => {
                // 基础分 0.7
                let mut score = 0.7;

                // 成功的工具调用比例加分
                if trace.metrics.tool_calls_count > 0 {
                    let success_rate = trace.metrics.successful_tool_calls as f64
                        / trace.metrics.tool_calls_count as f64;
                    score += success_rate * 0.2;
                }

                // 迭代次数少加分（效率）
                if trace.metrics.total_iterations <= 3 {
                    score += 0.1;
                }

                score.min(1.0)
            }
            ReactStatus::MaxIterationsReached => 0.3,
            ReactStatus::Failed => 0.1,
            ReactStatus::Cancelled => 0.2,
            ReactStatus::Running => 0.5,
        }
    }
}

/// Context Summarization 辅助结构
#[derive(Debug, Clone)]
pub struct ContextSummarizer {
    threshold: usize,
}

impl ContextSummarizer {
    pub fn new(threshold: usize) -> Self {
        Self { threshold }
    }

    /// 检查是否需要摘要
    pub fn needs_summarization(&self, history_len: usize) -> bool {
        history_len > self.threshold
    }

    /// 构建摘要提示词
    pub fn build_summarization_prompt(&self, history: &[String]) -> String {
        let history_text = history.join("\n---\n");

        format!(
            r#"Please summarize the following ReAct reasoning history concisely, preserving:
1. Key observations and findings
2. Important tool results
3. Critical decision points
4. Any errors encountered

History:
{}

Provide a concise summary (max 500 words) that captures the essential information for continuing the task."#,
            history_text
        )
    }

    /// 应用摘要到历史（替换旧的条目）
    pub fn apply_summary(&self, history: &mut Vec<String>, summary: String) {
        let keep_recent = self.threshold / 2;
        let to_summarize = history.len() - keep_recent;

        if to_summarize > 0 {
            // 移除旧条目
            history.drain(..to_summarize);
            // 插入摘要
            history.insert(0, format!("=== Previous Context Summary ===\n{}", summary));
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_cache_key_generation() {
        let integration = ReactMemoryIntegration::new(Arc::new(RwLock::new(
            IntelligentMemory::new(),
        )));

        let args1 = serde_json::json!({"url": "http://example.com"});
        let args2 = serde_json::json!({"url": "http://example.com"});
        let args3 = serde_json::json!({"url": "http://other.com"});

        let key1 = integration.build_cache_key("http_request", &args1);
        let key2 = integration.build_cache_key("http_request", &args2);
        let key3 = integration.build_cache_key("http_request", &args3);

        assert_eq!(key1, key2);
        assert_ne!(key1, key3);
    }

    #[test]
    fn test_confidence_score_calculation() {
        let integration = ReactMemoryIntegration::new(Arc::new(RwLock::new(
            IntelligentMemory::new(),
        )));

        let mut trace = ReactTrace::new("test task".to_string());
        trace.status = ReactStatus::Completed;
        trace.metrics.tool_calls_count = 5;
        trace.metrics.successful_tool_calls = 5;
        trace.metrics.total_iterations = 2;

        let score = integration.calculate_confidence_score(&trace);
        assert!(score > 0.9); // 完成 + 100%成功 + 少迭代
    }

    #[test]
    fn test_context_summarizer() {
        let summarizer = ContextSummarizer::new(5);

        assert!(!summarizer.needs_summarization(3));
        assert!(summarizer.needs_summarization(8));
    }
}

