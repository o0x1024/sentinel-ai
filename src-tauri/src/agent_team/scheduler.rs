//! Agent Team DAG 并发调度器
//! 支持多角色并发执行（Challenge 阶段）以及角色依赖图管理

use std::collections::{HashMap, HashSet};
use std::sync::Arc;
use anyhow::Result;
use futures::future::join_all;
use tokio::sync::Semaphore;

use super::models::AgentTeamMember;

/// 角色执行节点
#[derive(Debug, Clone)]
pub struct RoleNode {
    pub member_id: String,
    pub member_name: String,
    pub depends_on: Vec<String>, // 依赖的其他角色 member_id
}

/// DAG 调度计划：按层次执行
pub struct SchedulePlan {
    /// 每层可并发执行的 member_id 列表
    pub layers: Vec<Vec<String>>,
}

impl SchedulePlan {
    /// 将成员列表拓扑排序，生成执行层次
    /// 若无依赖配置，默认第一层（proposer）顺序执行，其余层并发
    pub fn build(members: &[AgentTeamMember], deps: Option<&HashMap<String, Vec<String>>>) -> Self {
        if members.is_empty() {
            return Self { layers: vec![] };
        }

        if let Some(dep_map) = deps {
            // 建立拓扑排序（Kahn's algorithm）
            let mut in_degree: HashMap<String, usize> = HashMap::new();
            let mut adj: HashMap<String, Vec<String>> = HashMap::new();

            for m in members {
                in_degree.entry(m.id.clone()).or_insert(0);
                if let Some(deps) = dep_map.get(&m.id) {
                    for dep in deps {
                        adj.entry(dep.clone()).or_default().push(m.id.clone());
                        *in_degree.entry(m.id.clone()).or_insert(0) += 1;
                    }
                }
            }

            let mut layers = vec![];
            let mut visited: HashSet<String> = HashSet::new();

            loop {
                let layer: Vec<String> = in_degree
                    .iter()
                    .filter(|(id, &deg)| deg == 0 && !visited.contains(*id))
                    .map(|(id, _)| id.clone())
                    .collect();

                if layer.is_empty() {
                    break;
                }

                for id in &layer {
                    visited.insert(id.clone());
                    if let Some(nexts) = adj.get(id) {
                        for next in nexts {
                            *in_degree.entry(next.clone()).or_default() -= 1;
                        }
                    }
                }
                layers.push(layer);
            }

            Self { layers }
        } else {
            // 默认：第 0 层为第一个成员（Proposer），其余并发
            let first = vec![members[0].id.clone()];
            if members.len() == 1 {
                Self { layers: vec![first] }
            } else {
                let rest: Vec<String> = members[1..].iter().map(|m| m.id.clone()).collect();
                Self { layers: vec![first, rest] }
            }
        }
    }
}

/// 并发执行结果
pub struct ConcurrentResult {
    pub member_id: String,
    pub output: Result<String>,
}

/// 并发执行一批角色任务（带并发限制）
/// 每个 `task` 是 (member_id, async closure -> Result<String>)
pub async fn run_concurrent_layer<F, Fut>(
    tasks: Vec<(String, F)>,
    max_concurrent: usize,
) -> Vec<ConcurrentResult>
where
    F: FnOnce() -> Fut + Send + 'static,
    Fut: std::future::Future<Output = Result<String>> + Send + 'static,
{
    let semaphore = Arc::new(Semaphore::new(max_concurrent));
    let mut handles = vec![];

    for (member_id, task_fn) in tasks {
        let sem = semaphore.clone();
        let mid = member_id.clone();
        let handle = tokio::spawn(async move {
            let _permit = sem.acquire_owned().await.expect("semaphore closed");
            let output = task_fn().await;
            ConcurrentResult {
                member_id: mid,
                output,
            }
        });
        handles.push(handle);
    }

    let results = join_all(handles).await;
    results
        .into_iter()
        .filter_map(|r| r.ok())
        .collect()
}

/// 分歧度量化器
pub struct DivergenceCalculator;

impl DivergenceCalculator {
    /// 计算多个角色 Review 之间的分歧度（0.0 = 完全一致，1.0 = 完全分歧）
    /// 采用基于关键词重叠的快速算法
    pub fn calculate(reviews: &[&str]) -> f64 {
        if reviews.len() < 2 {
            return 0.0;
        }

        // 1. 提取各个 review 的关键词集合
        let keyword_sets: Vec<HashSet<String>> = reviews
            .iter()
            .map(|text| extract_keywords(text))
            .collect();

        // 2. 计算 pairwise Jaccard 距离
        let mut total_distance = 0.0;
        let mut count = 0;

        for i in 0..keyword_sets.len() {
            for j in (i + 1)..keyword_sets.len() {
                let a = &keyword_sets[i];
                let b = &keyword_sets[j];
                let intersection = a.intersection(b).count();
                let union = a.union(b).count();
                let jaccard_similarity = if union == 0 {
                    1.0
                } else {
                    intersection as f64 / union as f64
                };
                total_distance += 1.0 - jaccard_similarity;
                count += 1;
            }
        }

        if count == 0 {
            return 0.0;
        }

        // 3. 加入情感倾向差异权重
        let sentiment_variance = calculate_sentiment_variance(reviews);
        let base_divergence = total_distance / count as f64;

        // 综合得分（60% 词汇差异 + 40% 情感差异）
        let divergence = base_divergence * 0.6 + sentiment_variance * 0.4;
        divergence.clamp(0.0, 1.0)
    }

    /// 判断是否需要人工介入
    pub fn needs_human_intervention(score: f64, threshold: f64) -> bool {
        score >= threshold
    }

    /// 判断是否需要额外的 Challenge 轮次
    pub fn needs_extra_challenge(score: f64, warn_threshold: f64) -> bool {
        score >= warn_threshold * 0.7
    }
}

/// 工具权限治理器
pub struct ToolGovernance {
    /// 全局黑名单（任意角色都不能使用）
    global_blacklist: HashSet<String>,
    /// 全局白名单（若设置，则只允许其中工具）
    global_allowlist: Option<HashSet<String>>,
    /// 角色级白名单 member_id -> allowed tools
    role_allowlist: HashMap<String, HashSet<String>>,
    /// 角色级黑名单 member_id -> denied tools
    role_denylist: HashMap<String, HashSet<String>>,
    /// 工具调用上限 member_id -> max_calls
    call_limits: HashMap<String, u32>,
    /// 全局调用上限（会话级）
    global_call_limit: Option<u32>,
    /// 当前调用计数 member_id -> tool_name -> count
    call_counts: HashMap<String, HashMap<String, u32>>,
    /// 高危工具集合（需要审批）
    sensitive_tools: HashSet<String>,
    /// 互斥锁：tool_name -> 当前持有者 member_id
    mutex_locks: HashMap<String, Option<String>>,
}

impl ToolGovernance {
    pub fn new() -> Self {
        Self {
            global_blacklist: HashSet::new(),
            global_allowlist: None,
            role_allowlist: HashMap::new(),
            role_denylist: HashMap::new(),
            call_limits: HashMap::new(),
            global_call_limit: None,
            call_counts: HashMap::new(),
            sensitive_tools: HashSet::new(),
            mutex_locks: HashMap::new(),
        }
    }

    /// 从会话级 tool_policy JSON 加载配置（作用于整个 Team）
    pub fn load_session_policy(&mut self, tool_policy: &serde_json::Value) {
        if let Some(enabled) = tool_policy.get("enabled").and_then(|v| v.as_bool()) {
            if !enabled {
                self.global_allowlist = Some(HashSet::new());
            }
        }

        if let Some(allowlist) = tool_policy.get("allowlist").and_then(|v| v.as_array()) {
            let tools: HashSet<String> = allowlist
                .iter()
                .filter_map(|v| v.as_str().map(|s| s.to_string()))
                .collect();
            self.global_allowlist = Some(tools);
        }

        if let Some(denylist) = tool_policy.get("denylist").and_then(|v| v.as_array()) {
            self.global_blacklist = denylist
                .iter()
                .filter_map(|v| v.as_str().map(|s| s.to_string()))
                .collect();
        }

        if let Some(max_calls) = tool_policy.get("max_calls").and_then(|v| v.as_u64()) {
            self.global_call_limit = Some(max_calls as u32);
        }

        if let Some(sensitive_tools) = tool_policy.get("sensitive_tools").and_then(|v| v.as_array()) {
            self.sensitive_tools = sensitive_tools
                .iter()
                .filter_map(|v| v.as_str().map(|s| s.to_string()))
                .collect();
        }
    }

    /// 从角色的 tool_policy JSON 加载配置
    pub fn load_member_policy(&mut self, member_id: &str, tool_policy: &serde_json::Value) {
        if let Some(allowlist) = tool_policy.get("allowlist").and_then(|v| v.as_array()) {
            let tools: HashSet<String> = allowlist
                .iter()
                .filter_map(|v| v.as_str().map(|s| s.to_string()))
                .collect();
            self.role_allowlist.insert(member_id.to_string(), tools);
        }
        if let Some(denylist) = tool_policy.get("denylist").and_then(|v| v.as_array()) {
            let tools: HashSet<String> = denylist
                .iter()
                .filter_map(|v| v.as_str().map(|s| s.to_string()))
                .collect();
            self.role_denylist.insert(member_id.to_string(), tools);
        }
        if let Some(max_calls) = tool_policy.get("max_calls").and_then(|v| v.as_u64()) {
            self.call_limits.insert(member_id.to_string(), max_calls as u32);
        }
    }

    /// 检查某个角色是否有权限使用某工具
    pub fn check_permission(&self, member_id: &str, tool_name: &str) -> ToolPermissionResult {
        // 全局白名单（如配置了白名单，则只允许白名单内的工具）
        if let Some(global_allowlist) = &self.global_allowlist {
            if !global_allowlist.contains(tool_name) {
                return ToolPermissionResult::Denied("不在会话工具白名单中".to_string());
            }
        }

        // 全局黑名单
        if self.global_blacklist.contains(tool_name) {
            return ToolPermissionResult::Denied("全局黑名单工具".to_string());
        }

        // 角色白名单（如配置了白名单，则只允许白名单内的工具）
        if let Some(allowlist) = self.role_allowlist.get(member_id) {
            if !allowlist.contains(tool_name) {
                return ToolPermissionResult::Denied("不在角色工具白名单中".to_string());
            }
        }

        // 角色黑名单
        if let Some(denylist) = self.role_denylist.get(member_id) {
            if denylist.contains(tool_name) {
                return ToolPermissionResult::Denied("在角色工具黑名单中".to_string());
            }
        }

        // 调用上限检查
        if let Some(max) = self.call_limits.get(member_id) {
            let counts = self.call_counts.get(member_id);
            let total: u32 = counts.map(|c| c.values().sum()).unwrap_or(0);
            if total >= *max {
                return ToolPermissionResult::Denied("已超出工具调用上限".to_string());
            }
        } else if let Some(global_max) = self.global_call_limit {
            let counts = self.call_counts.get(member_id);
            let total: u32 = counts.map(|c| c.values().sum()).unwrap_or(0);
            if total >= global_max {
                return ToolPermissionResult::Denied("已超出会话工具调用上限".to_string());
            }
        }

        // 高危工具需要审批
        if self.sensitive_tools.contains(tool_name) {
            return ToolPermissionResult::NeedsApproval;
        }

        ToolPermissionResult::Allowed
    }

    /// 尝试获取工具互斥锁（同轮次内单占）
    pub fn try_acquire_mutex(&mut self, tool_name: &str, member_id: &str) -> bool {
        match self.mutex_locks.get(tool_name) {
            None | Some(None) => {
                self.mutex_locks.insert(tool_name.to_string(), Some(member_id.to_string()));
                true
            }
            Some(Some(holder)) => holder == member_id,
        }
    }

    /// 释放工具互斥锁
    pub fn release_mutex(&mut self, tool_name: &str) {
        self.mutex_locks.insert(tool_name.to_string(), None);
    }

    /// 记录工具调用次数
    pub fn record_call(&mut self, member_id: &str, tool_name: &str) {
        let member_counts = self.call_counts.entry(member_id.to_string()).or_default();
        *member_counts.entry(tool_name.to_string()).or_insert(0) += 1;
    }

    /// 重置互斥锁（每轮结束时调用）
    pub fn reset_round_locks(&mut self) {
        for v in self.mutex_locks.values_mut() {
            *v = None;
        }
    }

    /// 获取调用统计摘要
    pub fn get_stats(&self, member_id: &str) -> HashMap<String, u32> {
        self.call_counts
            .get(member_id)
            .cloned()
            .unwrap_or_default()
    }
}

/// 工具权限检查结果
#[derive(Debug, PartialEq)]
pub enum ToolPermissionResult {
    Allowed,
    NeedsApproval,
    Denied(String),
}

// ==================== 私有辅助函数 ====================

/// 从文本提取关键词（中英文分词简化版）
fn extract_keywords(text: &str) -> HashSet<String> {
    let stop_words: HashSet<&str> = [
        "的", "了", "是", "在", "我", "有", "和", "就", "不", "人", "都", "一", "一个", "上",
        "也", "很", "到", "说", "要", "去", "你", "会", "着", "没有", "看", "好", "自己", "这",
        "the", "a", "an", "is", "are", "was", "were", "be", "been", "being", "have", "has",
        "had", "do", "does", "did", "will", "would", "could", "should", "may", "might",
        "that", "this", "these", "those", "with", "from", "they", "we", "you", "it",
    ]
    .iter()
    .copied()
    .collect();

    // 中文按字符，英文按词
    let mut keywords = HashSet::new();
    let mut word_buf = String::new();

    for ch in text.chars() {
        if ch.is_ascii_alphabetic() || ch.is_ascii_digit() {
            word_buf.push(ch.to_ascii_lowercase());
        } else {
            if word_buf.len() >= 3 && !stop_words.contains(word_buf.as_str()) {
                keywords.insert(word_buf.clone());
            }
            word_buf.clear();
            // 中文字符直接加入（2字以上）
            if ch as u32 > 0x4E00 {
                let s = ch.to_string();
                if !stop_words.contains(s.as_str()) {
                    keywords.insert(s);
                }
            }
        }
    }
    if word_buf.len() >= 3 && !stop_words.contains(word_buf.as_str()) {
        keywords.insert(word_buf);
    }

    keywords
}

/// 计算情感倾向方差（简化：统计积极/消极词占比方差）
fn calculate_sentiment_variance(reviews: &[&str]) -> f64 {
    let positive_markers = ["同意", "赞同", "支持", "可行", "good", "agree", "yes", "approve", "✓", "correct"];
    let negative_markers = ["反对", "问题", "风险", "不可行", "bad", "disagree", "no", "reject", "×", "incorrect", "concern"];

    let scores: Vec<f64> = reviews
        .iter()
        .map(|text| {
            let lower = text.to_lowercase();
            let pos = positive_markers.iter().filter(|&&m| lower.contains(m)).count() as f64;
            let neg = negative_markers.iter().filter(|&&m| lower.contains(m)).count() as f64;
            if pos + neg == 0.0 {
                0.5 // 中性
            } else {
                pos / (pos + neg) // 1.0 = 全正面, 0.0 = 全负面
            }
        })
        .collect();

    if scores.len() < 2 {
        return 0.0;
    }

    let mean = scores.iter().sum::<f64>() / scores.len() as f64;
    let variance = scores.iter().map(|s| (s - mean).powi(2)).sum::<f64>() / scores.len() as f64;
    variance.sqrt().clamp(0.0, 1.0)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_divergence_identical() {
        let reviews = vec!["同意这个方案，技术可行", "同意这个方案，技术可行"];
        let score = DivergenceCalculator::calculate(&reviews.iter().map(|s| s.as_str()).collect::<Vec<_>>());
        assert!(score < 0.3, "Identical reviews should have low divergence, got {}", score);
    }

    #[test]
    fn test_divergence_opposite() {
        let reviews = vec![
            "完全同意，这个方案非常好，技术可行，安全无虞，推荐实施",
            "完全反对，这个方案存在严重风险，技术不可行，安全漏洞，不应实施",
        ];
        let score = DivergenceCalculator::calculate(&reviews.iter().map(|s| s.as_str()).collect::<Vec<_>>());
        assert!(score > 0.3, "Opposite reviews should have high divergence, got {}", score);
    }

    #[test]
    fn test_tool_permission() {
        let mut gov = ToolGovernance::new();
        assert_eq!(gov.check_permission("m1", "web_search"), ToolPermissionResult::Allowed);
        assert_eq!(gov.check_permission("m1", "shell"), ToolPermissionResult::Allowed);
        gov.load_session_policy(&serde_json::json!({
            "allowlist": ["web_search"],
            "denylist": ["browser_control"]
        }));
        assert_eq!(gov.check_permission("m1", "shell"), ToolPermissionResult::Denied("不在会话工具白名单中".to_string()));
        assert_eq!(gov.check_permission("m1", "browser_control"), ToolPermissionResult::Denied("不在会话工具白名单中".to_string()));
    }

    #[test]
    fn test_schedule_plan_default() {
        let members: Vec<AgentTeamMember> = (0..4)
            .map(|i| AgentTeamMember {
                id: format!("m{}", i),
                session_id: "s1".to_string(),
                name: format!("Member {}", i),
                responsibility: None,
                system_prompt: None,
                decision_style: None,
                risk_preference: None,
                weight: 1.0,
                tool_policy: None,
                output_schema: None,
                sort_order: i as i32,
                token_usage: 0,
                tool_calls_count: 0,
                is_active: true,
                created_at: chrono::Utc::now(),
                updated_at: chrono::Utc::now(),
            })
            .collect();

        let plan = SchedulePlan::build(&members, None);
        assert_eq!(plan.layers.len(), 2, "Should have 2 layers");
        assert_eq!(plan.layers[0].len(), 1, "First layer has 1 member (proposer)");
        assert_eq!(plan.layers[1].len(), 3, "Second layer has 3 members (concurrent)");
    }
}
