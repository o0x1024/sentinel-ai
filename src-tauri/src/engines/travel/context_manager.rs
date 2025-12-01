//! 上下文管理器 - Token优化核心组件
//!
//! 负责上下文压缩、历史摘要、缓存管理

use super::types::*;
use lru::LruCache;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::num::NonZeroUsize;
use std::sync::Arc;
use std::time::{Duration, SystemTime};
use tokio::sync::RwLock;

/// 上下文管理器
pub struct ContextManager {
    config: ContextManagerConfig,
    /// 规划缓存 (task_hash -> DagPlan)
    plan_cache: Arc<RwLock<LruCache<String, CachedPlan>>>,
    /// 工具结果缓存
    result_cache: Arc<RwLock<LruCache<String, CachedResult>>>,
}

/// 缓存的计划
#[derive(Debug, Clone)]
struct CachedPlan {
    plan: DagPlan,
    created_at: SystemTime,
    ttl: Duration,
}

/// 缓存的结果
#[derive(Debug, Clone)]
struct CachedResult {
    result: serde_json::Value,
    created_at: SystemTime,
    ttl: Duration,
}

/// 历史记录条目
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HistoryEntry {
    /// 类型
    pub entry_type: HistoryEntryType,
    /// 内容
    pub content: String,
    /// 时间戳
    pub timestamp: SystemTime,
    /// 重要性 (0-10)
    pub importance: u8,
}

/// 历史记录类型
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum HistoryEntryType {
    Task,
    Observation,
    ToolCall,
    ToolResult,
    Error,
    Decision,
}

/// 压缩后的上下文
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CompressedContext {
    /// 任务描述
    pub task: String,
    /// 目标信息
    pub target: Option<String>,
    /// 历史摘要
    pub history_summary: String,
    /// 关键工具结果
    pub key_results: HashMap<String, serde_json::Value>,
    /// 当前状态
    pub current_state: String,
}

impl ContextManager {
    pub fn new(config: ContextManagerConfig) -> Self {
        let cache_size = NonZeroUsize::new(100).unwrap();
        Self {
            config,
            plan_cache: Arc::new(RwLock::new(LruCache::new(cache_size))),
            result_cache: Arc::new(RwLock::new(LruCache::new(cache_size))),
        }
    }

    /// 压缩上下文
    pub fn compress_context(
        &self,
        task: &str,
        context: &HashMap<String, serde_json::Value>,
        history: &[HistoryEntry],
    ) -> CompressedContext {
        // 提取目标
        let target = context
            .get("target")
            .and_then(|v| v.as_str())
            .map(|s| s.to_string());

        // 压缩历史
        let history_summary = self.compress_history(history);

        // 提取关键结果
        let key_results = self.extract_key_results(context);

        // 构建当前状态
        let current_state = self.build_current_state(context);

        CompressedContext {
            task: task.to_string(),
            target,
            history_summary,
            key_results,
            current_state,
        }
    }

    /// 压缩历史记录
    pub fn compress_history(&self, history: &[HistoryEntry]) -> String {
        if history.is_empty() {
            return String::new();
        }

        // 按重要性排序，保留最重要的条目
        let mut sorted: Vec<_> = history.iter().collect();
        sorted.sort_by(|a, b| b.importance.cmp(&a.importance));

        // 取前 N 条
        let max_entries = self.config.max_history_entries;
        let selected: Vec<_> = sorted.into_iter().take(max_entries).collect();

        // 生成摘要
        let summaries: Vec<String> = selected
            .iter()
            .map(|entry| {
                let type_str = match entry.entry_type {
                    HistoryEntryType::Task => "任务",
                    HistoryEntryType::Observation => "观察",
                    HistoryEntryType::ToolCall => "调用",
                    HistoryEntryType::ToolResult => "结果",
                    HistoryEntryType::Error => "错误",
                    HistoryEntryType::Decision => "决策",
                };
                // 截断长内容
                let content = if entry.content.len() > 100 {
                    format!("{}...", &entry.content[..100])
                } else {
                    entry.content.clone()
                };
                format!("[{}] {}", type_str, content)
            })
            .collect();

        summaries.join("\n")
    }

    /// 压缩工具结果
    pub fn compress_tool_result(&self, result: &serde_json::Value) -> serde_json::Value {
        match result {
            serde_json::Value::Object(obj) => {
                let mut compressed = serde_json::Map::new();
                
                for (key, value) in obj {
                    // 保留白名单字段
                    if self.config.preserve_fields.contains(key) {
                        compressed.insert(key.clone(), value.clone());
                        continue;
                    }

                    // 压缩长字符串
                    if let serde_json::Value::String(s) = value {
                        if s.len() > self.config.max_tool_result_length {
                            let truncated = format!(
                                "{}...(truncated {} chars)",
                                &s[..self.config.max_tool_result_length],
                                s.len() - self.config.max_tool_result_length
                            );
                            compressed.insert(key.clone(), serde_json::Value::String(truncated));
                            continue;
                        }
                    }

                    // 压缩长数组
                    if let serde_json::Value::Array(arr) = value {
                        if arr.len() > 10 {
                            let mut truncated: Vec<_> = arr.iter().take(10).cloned().collect();
                            truncated.push(serde_json::json!(format!("...(+{} more)", arr.len() - 10)));
                            compressed.insert(key.clone(), serde_json::Value::Array(truncated));
                            continue;
                        }
                    }

                    compressed.insert(key.clone(), value.clone());
                }

                serde_json::Value::Object(compressed)
            }
            serde_json::Value::String(s) => {
                if s.len() > self.config.max_tool_result_length {
                    serde_json::Value::String(format!(
                        "{}...(truncated)",
                        &s[..self.config.max_tool_result_length]
                    ))
                } else {
                    result.clone()
                }
            }
            serde_json::Value::Array(arr) => {
                if arr.len() > 20 {
                    let mut truncated: Vec<_> = arr.iter().take(20).cloned().collect();
                    truncated.push(serde_json::json!(format!("...(+{} more)", arr.len() - 20)));
                    serde_json::Value::Array(truncated)
                } else {
                    result.clone()
                }
            }
            _ => result.clone(),
        }
    }

    /// 提取关键结果
    fn extract_key_results(
        &self,
        context: &HashMap<String, serde_json::Value>,
    ) -> HashMap<String, serde_json::Value> {
        let mut key_results = HashMap::new();

        // 保留关键字段
        for field in &self.config.preserve_fields {
            if let Some(value) = context.get(field) {
                key_results.insert(field.clone(), value.clone());
            }
        }

        // 查找步骤结果
        for (key, value) in context {
            if key.starts_with("step_") && key.ends_with("_result") {
                // 压缩步骤结果
                key_results.insert(key.clone(), self.compress_tool_result(value));
            }
        }

        key_results
    }

    /// 构建当前状态描述
    fn build_current_state(&self, context: &HashMap<String, serde_json::Value>) -> String {
        let mut state_parts = Vec::new();

        // 检查执行状态
        if let Some(status) = context.get("execution_status").and_then(|v| v.as_str()) {
            state_parts.push(format!("状态: {}", status));
        }

        // 检查已完成步骤
        if let Some(completed) = context.get("completed_steps").and_then(|v| v.as_array()) {
            state_parts.push(format!("已完成: {} 步", completed.len()));
        }

        // 检查错误
        if let Some(errors) = context.get("errors").and_then(|v| v.as_array()) {
            if !errors.is_empty() {
                state_parts.push(format!("错误: {} 个", errors.len()));
            }
        }

        if state_parts.is_empty() {
            "初始状态".to_string()
        } else {
            state_parts.join(", ")
        }
    }

    /// 缓存计划
    pub async fn cache_plan(&self, task_hash: &str, plan: DagPlan, ttl_secs: u64) {
        let cached = CachedPlan {
            plan,
            created_at: SystemTime::now(),
            ttl: Duration::from_secs(ttl_secs),
        };
        let mut cache = self.plan_cache.write().await;
        cache.put(task_hash.to_string(), cached);
        log::info!("ContextManager: Cached plan for task hash: {}", task_hash);
    }

    /// 获取缓存的计划
    pub async fn get_cached_plan(&self, task_hash: &str) -> Option<DagPlan> {
        let mut cache = self.plan_cache.write().await;
        if let Some(cached) = cache.get(task_hash) {
            // 检查是否过期
            if let Ok(elapsed) = cached.created_at.elapsed() {
                if elapsed < cached.ttl {
                    log::info!("ContextManager: Cache hit for task hash: {}", task_hash);
                    return Some(cached.plan.clone());
                }
            }
        }
        None
    }

    /// 生成任务哈希 (用于缓存键)
    pub fn generate_task_hash(task: &str, context: &HashMap<String, serde_json::Value>) -> String {
        use std::collections::hash_map::DefaultHasher;
        use std::hash::{Hash, Hasher};

        let mut hasher = DefaultHasher::new();
        task.hash(&mut hasher);

        // 包含关键上下文字段
        if let Some(target) = context.get("target") {
            target.to_string().hash(&mut hasher);
        }
        if let Some(tools) = context.get("tools_allow") {
            tools.to_string().hash(&mut hasher);
        }

        format!("{:x}", hasher.finish())
    }

    /// 估算 Token 数量 (简单估算: 约4字符=1 token)
    pub fn estimate_tokens(text: &str) -> usize {
        text.len() / 4
    }

    /// 检查是否超过 Token 限制
    pub fn is_over_limit(&self, text: &str) -> bool {
        Self::estimate_tokens(text) > self.config.max_context_tokens
    }

    /// 创建精简的用户提示 (用于 DAG 规划)
    pub fn create_lite_user_prompt(&self, compressed: &CompressedContext) -> String {
        let mut prompt = format!("任务: {}\n", compressed.task);

        if let Some(target) = &compressed.target {
            prompt.push_str(&format!("目标: {}\n", target));
        }

        if !compressed.history_summary.is_empty() {
            prompt.push_str(&format!("\n历史:\n{}\n", compressed.history_summary));
        }

        if !compressed.key_results.is_empty() {
            prompt.push_str("\n已知信息:\n");
            for (key, value) in &compressed.key_results {
                let value_str = match value {
                    serde_json::Value::String(s) => s.clone(),
                    _ => value.to_string(),
                };
                // 截断长值
                let display_value = if value_str.len() > 100 {
                    format!("{}...", &value_str[..100])
                } else {
                    value_str
                };
                prompt.push_str(&format!("- {}: {}\n", key, display_value));
            }
        }

        prompt.push_str(&format!("\n当前状态: {}", compressed.current_state));

        prompt
    }
}

impl Default for ContextManager {
    fn default() -> Self {
        Self::new(ContextManagerConfig::default())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_compress_tool_result() {
        let config = ContextManagerConfig {
            max_tool_result_length: 50,
            ..Default::default()
        };
        let manager = ContextManager::new(config);

        // 测试长字符串压缩
        let long_result = serde_json::json!({
            "data": "a".repeat(100),
            "status": "success"
        });

        let compressed = manager.compress_tool_result(&long_result);
        let data = compressed.get("data").unwrap().as_str().unwrap();
        assert!(data.contains("truncated"));
        
        // status 在保留字段中
        assert_eq!(
            compressed.get("status").unwrap().as_str().unwrap(),
            "success"
        );
    }

    #[test]
    fn test_generate_task_hash() {
        let mut context1 = HashMap::new();
        context1.insert("target".to_string(), serde_json::json!("example.com"));

        let mut context2 = HashMap::new();
        context2.insert("target".to_string(), serde_json::json!("example.com"));

        let hash1 = ContextManager::generate_task_hash("scan target", &context1);
        let hash2 = ContextManager::generate_task_hash("scan target", &context2);

        assert_eq!(hash1, hash2);
    }

    #[test]
    fn test_estimate_tokens() {
        let text = "Hello world"; // 11 chars
        assert_eq!(ContextManager::estimate_tokens(text), 2);
    }
}

