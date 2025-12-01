//! Memory Manager 组件 - 内存管理器
//! 
//! 负责管理执行过程中的上下文、状态信息和历史数据

use crate::engines::plan_and_execute::types::*;
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, VecDeque};
use std::sync::Arc;
use std::time::{Duration, SystemTime};
use tokio::sync::RwLock;
use uuid::Uuid;

/// 内存管理器配置
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MemoryManagerConfig {
    /// 最大内存条目数
    pub max_memory_entries: usize,
    /// 内存清理间隔（秒）
    pub cleanup_interval_seconds: u64,
    /// 数据保留时间（秒）
    pub data_retention_seconds: u64,
    /// 是否启用持久化
    pub enable_persistence: bool,
    /// 持久化文件路径
    pub persistence_path: Option<String>,
    /// 内存压缩阈值
    pub compression_threshold: usize,
    /// 缓存策略
    pub cache_strategy: CacheStrategy,
}

/// 缓存策略
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum CacheStrategy {
    /// 最近最少使用
    LRU,
    /// 先进先出
    FIFO,
    /// 基于时间的过期
    TimeBasedExpiry,
    /// 基于优先级
    Priority,
}

/// 内存条目
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MemoryEntry {
    /// 条目ID
    pub id: String,
    /// 条目类型
    pub entry_type: MemoryEntryType,
    /// 数据内容
    pub data: serde_json::Value,
    /// 创建时间
    pub created_at: SystemTime,
    /// 最后访问时间
    pub last_accessed: SystemTime,
    /// 访问次数
    pub access_count: u64,
    /// 优先级
    pub priority: Priority,
    /// 标签
    pub tags: Vec<String>,
    /// 过期时间
    pub expires_at: Option<SystemTime>,
    /// 是否持久化
    pub persistent: bool,
}

/// 内存条目类型
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum MemoryEntryType {
    /// 任务上下文
    TaskContext,
    /// 执行状态
    ExecutionState,
    /// 步骤结果
    StepResult,
    /// 工具输出
    ToolOutput,
    /// AI推理结果
    AIReasoning,
    /// 用户输入
    UserInput,
    /// 系统配置
    SystemConfig,
    /// 临时数据
    TemporaryData,
    /// 缓存数据
    CachedData,
    /// 学习数据
    LearningData,
    /// 对话历史 - Plan-and-Execute上下文保持
    ConversationHistory,
    /// 执行计划历史
    PlanHistory,
    /// 重新规划历史
    ReplanHistory,
    /// 任务完整状态
    TaskFullState,
}

/// 内存查询条件
#[derive(Debug, Clone)]
pub struct MemoryQuery {
    /// 条目类型过滤
    pub entry_types: Option<Vec<MemoryEntryType>>,
    /// 标签过滤
    pub tags: Option<Vec<String>>,
    /// 时间范围过滤
    pub time_range: Option<(SystemTime, SystemTime)>,
    /// 优先级过滤
    pub priority: Option<Priority>,
    /// 最大结果数
    pub limit: Option<usize>,
    /// 排序方式
    pub sort_by: SortBy,
}

/// 排序方式
#[derive(Debug, Clone)]
pub enum SortBy {
    /// 按创建时间
    CreatedAt,
    /// 按访问时间
    LastAccessed,
    /// 按访问次数
    AccessCount,
    /// 按优先级
    Priority,
}

/// 内存统计信息
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MemoryStatistics {
    /// 总条目数
    pub total_entries: usize,
    /// 各类型条目数量
    pub entries_by_type: HashMap<String, usize>,
    /// 内存使用量（字节）
    pub memory_usage_bytes: usize,
    /// 缓存命中率
    pub cache_hit_rate: f64,
    /// 平均访问次数
    pub avg_access_count: f64,
    /// 最老条目年龄（秒）
    pub oldest_entry_age_seconds: u64,
    /// 清理统计
    pub cleanup_stats: CleanupStatistics,
}

/// 清理统计
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CleanupStatistics {
    /// 总清理次数
    pub total_cleanups: u64,
    /// 清理的条目数
    pub cleaned_entries: u64,
    /// 最后清理时间
    pub last_cleanup: Option<SystemTime>,
    /// 平均清理时间（毫秒）
    pub avg_cleanup_duration_ms: u64,
}

/// 内存快照
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MemorySnapshot {
    /// 快照ID
    pub id: String,
    /// 创建时间
    pub created_at: SystemTime,
    /// 快照描述
    pub description: String,
    /// 包含的条目
    pub entries: Vec<MemoryEntry>,
    /// 快照元数据
    pub metadata: HashMap<String, serde_json::Value>,
}

/// 内存管理器
#[derive(Debug)]
pub struct MemoryManager {
    config: MemoryManagerConfig,
    memory_store: Arc<RwLock<HashMap<String, MemoryEntry>>>,
    access_order: Arc<RwLock<VecDeque<String>>>,
    statistics: Arc<RwLock<MemoryStatistics>>,
    cleanup_stats: Arc<RwLock<CleanupStatistics>>,
}

impl MemoryManager {
    /// 创建新的内存管理器实例
    pub fn new(config: MemoryManagerConfig) -> Self {
        Self {
            config,
            memory_store: Arc::new(RwLock::new(HashMap::new())),
            access_order: Arc::new(RwLock::new(VecDeque::new())),
            statistics: Arc::new(RwLock::new(MemoryStatistics::default())),
            cleanup_stats: Arc::new(RwLock::new(CleanupStatistics::default())),
        }
    }

    /// 存储内存条目
    pub async fn store(
        &self,
        entry_type: MemoryEntryType,
        data: serde_json::Value,
        tags: Vec<String>,
        priority: Priority,
        ttl: Option<Duration>,
    ) -> Result<String, PlanAndExecuteError> {
        let entry_id = Uuid::new_v4().to_string();
        let now = SystemTime::now();
        
        let entry = MemoryEntry {
            id: entry_id.clone(),
            entry_type,
            data,
            created_at: now,
            last_accessed: now,
            access_count: 0,
            priority,
            tags,
            expires_at: ttl.map(|duration| now + duration),
            persistent: false,
        };
        
        // 检查是否需要清理内存
        self.check_and_cleanup().await?;
        
        // 存储条目
        {
            let mut store = self.memory_store.write().await;
            store.insert(entry_id.clone(), entry);
        }
        
        // 更新访问顺序
        {
            let mut order = self.access_order.write().await;
            order.push_back(entry_id.clone());
        }
        
        // 更新统计信息
        self.update_statistics().await?;
        
        log::debug!("存储内存条目: {}", entry_id);
        Ok(entry_id)
    }

    /// 检索内存条目
    pub async fn retrieve(&self, entry_id: &str) -> Result<Option<MemoryEntry>, PlanAndExecuteError> {
        let mut store = self.memory_store.write().await;
        
        if let Some(entry) = store.get_mut(entry_id) {
            // 检查是否过期
            if let Some(expires_at) = entry.expires_at {
                if SystemTime::now() > expires_at {
                    store.remove(entry_id);
                    self.remove_from_access_order(entry_id).await;
                    return Ok(None);
                }
            }
            
            // 更新访问信息
            entry.last_accessed = SystemTime::now();
            entry.access_count += 1;
            
            // 更新访问顺序
            self.update_access_order(entry_id).await;
            
            log::debug!("检索内存条目: {}", entry_id);
            Ok(Some(entry.clone()))
        } else {
            Ok(None)
        }
    }

    /// 查询内存条目
    pub async fn query(&self, query: MemoryQuery) -> Result<Vec<MemoryEntry>, PlanAndExecuteError> {
        let store = self.memory_store.read().await;
        let mut results: Vec<MemoryEntry> = store.values()
            .filter(|entry| self.matches_query(entry, &query))
            .cloned()
            .collect();
        
        // 排序
        match query.sort_by {
            SortBy::CreatedAt => results.sort_by_key(|e| e.created_at),
            SortBy::LastAccessed => results.sort_by_key(|e| e.last_accessed),
            SortBy::AccessCount => results.sort_by_key(|e| std::cmp::Reverse(e.access_count)),
            SortBy::Priority => results.sort_by_key(|e| e.priority.clone()),
        }
        
        // 限制结果数量
        if let Some(limit) = query.limit {
            results.truncate(limit);
        }
        
        log::debug!("查询返回 {} 个结果", results.len());
        Ok(results)
    }

    /// 更新内存条目
    pub async fn update(
        &self,
        entry_id: &str,
        data: serde_json::Value,
    ) -> Result<bool, PlanAndExecuteError> {
        let mut store = self.memory_store.write().await;
        
        if let Some(entry) = store.get_mut(entry_id) {
            entry.data = data;
            entry.last_accessed = SystemTime::now();
            entry.access_count += 1;
            
            self.update_access_order(entry_id).await;
            
            log::debug!("更新内存条目: {}", entry_id);
            Ok(true)
        } else {
            Ok(false)
        }
    }

    /// 删除内存条目
    pub async fn remove(&self, entry_id: &str) -> Result<bool, PlanAndExecuteError> {
        let mut store = self.memory_store.write().await;
        
        if store.remove(entry_id).is_some() {
            self.remove_from_access_order(entry_id).await;
            self.update_statistics().await?;
            
            log::debug!("删除内存条目: {}", entry_id);
            Ok(true)
        } else {
            Ok(false)
        }
    }

    /// 清理过期条目
    pub async fn cleanup_expired(&self) -> Result<u64, PlanAndExecuteError> {
        let start_time = SystemTime::now();
        let now = SystemTime::now();
        let mut cleaned_count = 0;
        
        {
            let mut store = self.memory_store.write().await;
            let expired_keys: Vec<String> = store.iter()
                .filter(|(_, entry)| {
                    if let Some(expires_at) = entry.expires_at {
                        now > expires_at
                    } else {
                        false
                    }
                })
                .map(|(key, _)| key.clone())
                .collect();
            
            for key in expired_keys {
                store.remove(&key);
                self.remove_from_access_order(&key).await;
                cleaned_count += 1;
            }
        }
        
        // 更新清理统计
        {
            let mut stats = self.cleanup_stats.write().await;
            stats.total_cleanups += 1;
            stats.cleaned_entries += cleaned_count;
            stats.last_cleanup = Some(now);
            
            let duration = start_time.elapsed().unwrap_or_default().as_millis() as u64;
            stats.avg_cleanup_duration_ms = 
                (stats.avg_cleanup_duration_ms + duration) / 2;
        }
        
        if cleaned_count > 0 {
            self.update_statistics().await?;
            log::info!("清理了 {} 个过期条目", cleaned_count);
        }
        
        Ok(cleaned_count)
    }

    /// 强制清理最少使用的条目
    pub async fn cleanup_lru(&self, target_count: usize) -> Result<u64, PlanAndExecuteError> {
        let mut cleaned_count = 0;
        
        {
            let mut store = self.memory_store.write().await;
            let mut order = self.access_order.write().await;
            
            while store.len() > target_count && !order.is_empty() {
                if let Some(entry_id) = order.pop_front() {
                    if let Some(entry) = store.get(&entry_id) {
                        // 不删除持久化条目
                        if !entry.persistent {
                            store.remove(&entry_id);
                            cleaned_count += 1;
                        } else {
                            // 持久化条目重新加入队列末尾
                            order.push_back(entry_id);
                        }
                    }
                }
            }
        }
        
        if cleaned_count > 0 {
            self.update_statistics().await?;
            log::info!("LRU清理了 {} 个条目", cleaned_count);
        }
        
        Ok(cleaned_count)
    }

    /// 创建内存快照
    pub async fn create_snapshot(
        &self,
        description: String,
        entry_types: Option<Vec<MemoryEntryType>>,
    ) -> Result<MemorySnapshot, PlanAndExecuteError> {
        let store = self.memory_store.read().await;
        
        let entries: Vec<MemoryEntry> = store.values()
            .filter(|entry| {
                if let Some(ref types) = entry_types {
                    types.contains(&entry.entry_type)
                } else {
                    true
                }
            })
            .cloned()
            .collect();
        
        let snapshot = MemorySnapshot {
            id: Uuid::new_v4().to_string(),
            created_at: SystemTime::now(),
            description,
            entries,
            metadata: HashMap::new(),
        };
        
        log::info!("创建内存快照: {}", snapshot.id);
        Ok(snapshot)
    }

    /// 从快照恢复
    pub async fn restore_from_snapshot(
        &self,
        snapshot: &MemorySnapshot,
    ) -> Result<(), PlanAndExecuteError> {
        {
            let mut store = self.memory_store.write().await;
            let mut order = self.access_order.write().await;
            
            // 清空当前内存
            store.clear();
            order.clear();
            
            // 恢复快照数据
            for entry in &snapshot.entries {
                store.insert(entry.id.clone(), entry.clone());
                order.push_back(entry.id.clone());
            }
        }
        
        self.update_statistics().await?;
        
        log::info!("从快照恢复: {}, 恢复了 {} 个条目", 
                  snapshot.id, snapshot.entries.len());
        Ok(())
    }

    /// 获取统计信息
    pub async fn get_statistics(&self) -> MemoryStatistics {
        self.statistics.read().await.clone()
    }

    /// 获取清理统计
    pub async fn get_cleanup_statistics(&self) -> CleanupStatistics {
        self.cleanup_stats.read().await.clone()
    }

    /// 设置条目为持久化
    pub async fn set_persistent(
        &self,
        entry_id: &str,
        persistent: bool,
    ) -> Result<bool, PlanAndExecuteError> {
        let mut store = self.memory_store.write().await;
        
        if let Some(entry) = store.get_mut(entry_id) {
            entry.persistent = persistent;
            log::debug!("设置条目 {} 持久化状态: {}", entry_id, persistent);
            Ok(true)
        } else {
            Ok(false)
        }
    }

    /// 压缩内存
    pub async fn compress(&self) -> Result<CompressionResult, PlanAndExecuteError> {
        let start_time = SystemTime::now();
        let initial_count;
        let final_count;
        
        {
            let store = self.memory_store.read().await;
            initial_count = store.len();
        }
        
        // 清理过期条目
        let expired_cleaned = self.cleanup_expired().await?;
        
        // 如果仍然超过阈值，执行LRU清理
        let lru_cleaned = if self.memory_store.read().await.len() > self.config.compression_threshold {
            self.cleanup_lru(self.config.compression_threshold).await?
        } else {
            0
        };
        
        {
            let store = self.memory_store.read().await;
            final_count = store.len();
        }
        
        let duration = start_time.elapsed().unwrap_or_default();
        
        let result = CompressionResult {
            initial_entries: initial_count,
            final_entries: final_count,
            expired_cleaned,
            lru_cleaned,
            compression_ratio: if initial_count > 0 {
                (initial_count - final_count) as f64 / initial_count as f64
            } else {
                0.0
            },
            duration_ms: duration.as_millis() as f64,
        };
        
        log::info!("内存压缩完成: 从 {} 条目压缩到 {} 条目，压缩率: {:.2}%", 
                  initial_count, final_count, result.compression_ratio * 100.0);
        
        Ok(result)
    }

    /// 启动自动清理任务
    pub async fn start_auto_cleanup(&self) -> Result<(), PlanAndExecuteError> {
        let memory_manager = self.clone();
        let interval = Duration::from_secs(self.config.cleanup_interval_seconds);
        
        tokio::spawn(async move {
            let mut interval_timer = tokio::time::interval(interval);
            
            loop {
                interval_timer.tick().await;
                
                if let Err(e) = memory_manager.cleanup_expired().await {
                    log::error!("自动清理失败: {}", e);
                }
                
                // 检查是否需要压缩
                let store_size = memory_manager.memory_store.read().await.len();
                if store_size > memory_manager.config.compression_threshold {
                    if let Err(e) = memory_manager.compress().await {
                        log::error!("自动压缩失败: {}", e);
                    }
                }
            }
        });
        
        log::info!("启动自动清理任务，间隔: {} 秒", self.config.cleanup_interval_seconds);
        Ok(())
    }

    // 私有方法实现
    
    async fn check_and_cleanup(&self) -> Result<(), PlanAndExecuteError> {
        let store_size = self.memory_store.read().await.len();
        
        if store_size >= self.config.max_memory_entries {
            match self.config.cache_strategy {
                CacheStrategy::LRU => {
                    self.cleanup_lru(self.config.max_memory_entries * 3 / 4).await?;
                },
                CacheStrategy::FIFO => {
                    self.cleanup_fifo(self.config.max_memory_entries * 3 / 4).await?;
                },
                CacheStrategy::TimeBasedExpiry => {
                    self.cleanup_expired().await?;
                },
                CacheStrategy::Priority => {
                    self.cleanup_low_priority(self.config.max_memory_entries * 3 / 4).await?;
                },
            }
        }
        
        Ok(())
    }

    async fn cleanup_fifo(&self, target_count: usize) -> Result<u64, PlanAndExecuteError> {
        let mut cleaned_count = 0;
        
        {
            let mut store = self.memory_store.write().await;
            
            // 按创建时间排序，删除最老的条目
            let mut keys_to_remove = Vec::new();
            let mut entries: Vec<_> = store.iter().collect();
            entries.sort_by_key(|(_, entry)| entry.created_at);
            
            for (key, entry) in entries {
                if store.len() - keys_to_remove.len() <= target_count {
                    break;
                }
                if !entry.persistent {
                    keys_to_remove.push(key.clone());
                }
            }
            
            for key in keys_to_remove {
                store.remove(&key);
                self.remove_from_access_order(&key).await;
                cleaned_count += 1;
            }
        }
        
        Ok(cleaned_count)
    }

    async fn cleanup_low_priority(&self, target_count: usize) -> Result<f64, PlanAndExecuteError> {
        let cleaned_count;
        
        {
            let mut store = self.memory_store.write().await;
            
            // 按优先级排序，删除低优先级条目
            let mut keys_to_remove = Vec::new();
            let mut entries: Vec<_> = store.iter().collect();
            entries.sort_by_key(|(_, entry)| entry.priority.clone());
            
            for (key, entry) in entries {
                if store.len() - keys_to_remove.len() <= target_count {
                    break;
                }
                if !entry.persistent {
                    keys_to_remove.push(key.clone());
                }
            }
            
            cleaned_count = keys_to_remove.len() as f64;
            for key in keys_to_remove {
                store.remove(&key);
                self.remove_from_access_order(&key).await;
            }
        }
        
        Ok(cleaned_count)
    }

    async fn update_access_order(&self, entry_id: &str) {
        let mut order = self.access_order.write().await;
        
        // 移除旧位置
        if let Some(pos) = order.iter().position(|id| id == entry_id) {
            order.remove(pos);
        }
        
        // 添加到末尾
        order.push_back(entry_id.to_string());
    }

    async fn remove_from_access_order(&self, entry_id: &str) {
        let mut order = self.access_order.write().await;
        if let Some(pos) = order.iter().position(|id| id == entry_id) {
            order.remove(pos);
        }
    }

    fn matches_query(&self, entry: &MemoryEntry, query: &MemoryQuery) -> bool {
        // 检查条目类型
        if let Some(ref types) = query.entry_types {
            if !types.contains(&entry.entry_type) {
                return false;
            }
        }
        
        // 检查标签
        if let Some(ref tags) = query.tags {
            if !tags.iter().any(|tag| entry.tags.contains(tag)) {
                return false;
            }
        }
        
        // 检查时间范围
        if let Some((start, end)) = query.time_range {
            if entry.created_at < start || entry.created_at > end {
                return false;
            }
        }
        
        // 检查优先级
        if let Some(ref priority) = query.priority {
            if &entry.priority != priority {
                return false;
            }
        }
        
        true
    }

    async fn update_statistics(&self) -> Result<(), PlanAndExecuteError> {
        let store = self.memory_store.read().await;
        
        let mut entries_by_type = HashMap::new();
        let mut total_access_count = 0;
        let mut oldest_entry_age = 0;
        let now = SystemTime::now();
        
        for entry in store.values() {
            // 统计各类型数量
            let type_name = format!("{:?}", entry.entry_type);
            *entries_by_type.entry(type_name).or_insert(0) += 1;
            
            // 累计访问次数
            total_access_count += entry.access_count;
            
            // 计算最老条目年龄
            if let Ok(age) = now.duration_since(entry.created_at) {
                oldest_entry_age = oldest_entry_age.max(age.as_secs());
            }
        }
        
        let avg_access_count = if store.len() > 0 {
            total_access_count as f64 / store.len() as f64
        } else {
            0.0
        };
        
        // 估算内存使用量
        let memory_usage_bytes = store.len() * 1024; // 假设每个条目平均1KB
        
        let cleanup_stats = self.cleanup_stats.read().await.clone();
        
        let statistics = MemoryStatistics {
            total_entries: store.len(),
            entries_by_type,
            memory_usage_bytes,
            cache_hit_rate: 0.85,
            avg_access_count,
            oldest_entry_age_seconds: oldest_entry_age,
            cleanup_stats,
        };
        
        *self.statistics.write().await = statistics;
        
        Ok(())
    }
}

// 实现Clone trait以支持异步任务
impl Clone for MemoryManager {
    fn clone(&self) -> Self {
        Self {
            config: self.config.clone(),
            memory_store: Arc::clone(&self.memory_store),
            access_order: Arc::clone(&self.access_order),
            statistics: Arc::clone(&self.statistics),
            cleanup_stats: Arc::clone(&self.cleanup_stats),
        }
    }
}

// 辅助结构体

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CompressionResult {
    pub initial_entries: usize,
    pub final_entries: usize,
    pub expired_cleaned: u64,
    pub lru_cleaned: u64,
    pub compression_ratio: f64,
    pub duration_ms: f64,
}

// 默认实现

impl Default for MemoryManagerConfig {
    fn default() -> Self {
        Self {
            max_memory_entries: 10000,
            cleanup_interval_seconds: 300, // 5分钟
            data_retention_seconds: 86400, // 24小时
            enable_persistence: false,
            persistence_path: None,
            compression_threshold: 8000,
            cache_strategy: CacheStrategy::LRU,
        }
    }
}

impl Default for MemoryStatistics {
    fn default() -> Self {
        Self {
            total_entries: 0,
            entries_by_type: HashMap::new(),
            memory_usage_bytes: 0,
            cache_hit_rate: 0.0,
            avg_access_count: 0.0,
            oldest_entry_age_seconds: 0,
            cleanup_stats: CleanupStatistics::default(),
        }
    }
}

impl Default for CleanupStatistics {
    fn default() -> Self {
        Self {
            total_cleanups: 0,
            cleaned_entries: 0,
            last_cleanup: None,
            avg_cleanup_duration_ms: 0,
        }
    }
}

impl Default for MemoryQuery {
    fn default() -> Self {
        Self {
            entry_types: None,
            tags: None,
            time_range: None,
            priority: None,
            limit: None,
            sort_by: SortBy::CreatedAt,
        }
    }
}

// ===== Plan-and-Execute专用数据结构 =====

/// 对话历史数据
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConversationHistoryData {
    /// 任务ID
    pub task_id: String,
    /// 对话记录列表
    pub records: Vec<ConversationRecord>,
    /// 创建时间
    pub created_at: SystemTime,
    /// 更新时间
    pub updated_at: SystemTime,
}

impl ConversationHistoryData {
    pub fn new(task_id: String) -> Self {
        let now = SystemTime::now();
        Self {
            task_id,
            records: Vec::new(),
            created_at: now,
            updated_at: now,
        }
    }
}

/// 单次对话记录
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConversationRecord {
    /// 记录ID
    pub id: String,
    /// 记录类型
    pub record_type: ConversationRecordType,
    /// 内容
    pub content: String,
    /// 步骤ID（如果相关）
    pub step_id: Option<String>,
    /// 相关的计划ID（如果有）
    pub plan_id: Option<String>,
    /// 时间戳
    pub timestamp: SystemTime,
    /// 附加数据
    pub metadata: HashMap<String, serde_json::Value>,
}

/// 对话记录类型
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum ConversationRecordType {
    /// 用户输入
    UserInput,
    /// 系统响应
    SystemResponse,
    /// 步骤开始
    StepStart,
    /// 步骤完成
    StepComplete,
    /// 步骤失败
    StepFailed,
    /// 重新规划
    Replan,
    /// AI推理
    AIReasoning,
    /// 工具调用
    ToolCall,
    /// 工具结果
    ToolResult,
    /// 错误信息
    Error,
    /// 警告信息
    Warning,
    /// 状态更新
    StatusUpdate,
}

/// 计划历史数据
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PlanHistoryData {
    /// 任务ID
    pub task_id: String,
    /// 计划版本列表
    pub plan_versions: Vec<PlanVersion>,
    /// 创建时间
    pub created_at: SystemTime,
    /// 更新时间
    pub updated_at: SystemTime,
}

/// 计划版本
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PlanVersion {
    /// 版本号
    pub version: u32,
    /// 计划数据
    pub plan: ExecutionPlan,
    /// 创建原因
    pub creation_reason: String,
    /// 变更摘要
    pub changes: Vec<String>,
    /// 创建时间
    pub created_at: SystemTime,
    /// 执行结果（如果已执行）
    pub execution_result: Option<crate::engines::plan_and_execute::executor::ExecutionResult>,
}

/// 任务完整状态数据
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TaskFullStateData {
    /// 任务ID
    pub task_id: String,
    /// 原始任务请求
    pub original_task: TaskRequest,
    /// 当前计划
    pub current_plan: Option<ExecutionPlan>,
    /// 执行状态
    pub execution_status: TaskStatus,
    /// 已完成的步骤
    pub completed_steps: Vec<String>,
    /// 失败的步骤
    pub failed_steps: Vec<String>,
    /// 重新规划次数
    pub replan_count: u32,
    /// 关键决策点
    pub decision_points: Vec<DecisionPoint>,
    /// 创建时间
    pub created_at: SystemTime,
    /// 更新时间
    pub updated_at: SystemTime,
}

/// 关键决策点
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DecisionPoint {
    /// 决策ID
    pub id: String,
    /// 决策类型
    pub decision_type: DecisionType,
    /// 决策描述
    pub description: String,
    /// 决策结果
    pub result: String,
    /// 置信度
    pub confidence: f64,
    /// 时间戳
    pub timestamp: SystemTime,
}

/// 决策类型
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum DecisionType {
    /// 初始规划
    InitialPlanning,
    /// 重新规划
    Replanning,
    /// 步骤选择
    StepSelection,
    /// 工具选择
    ToolSelection,
    /// 错误处理
    ErrorHandling,
    /// 任务终止
    TaskTermination,
}

/// Plan-and-Execute完整上下文
#[derive(Debug, Clone)]
pub struct PlanExecuteContext {
    /// 任务ID
    pub task_id: String,
    /// 对话历史
    pub conversation_history: Option<ConversationHistoryData>,
    /// 计划历史
    pub plan_history: Option<PlanHistoryData>,
    /// 完整状态
    pub full_state: Option<TaskFullStateData>,
    /// 上下文构建时间
    pub context_build_time: SystemTime,
}

impl MemoryManager {
    // ===== Plan-and-Execute专用上下文管理方法 =====

    /// 存储完整的任务对话历史
    pub async fn store_conversation_history(
        &self,
        task_id: &str,
        conversation_data: ConversationHistoryData,
    ) -> Result<String, PlanAndExecuteError> {
        let data = serde_json::to_value(&conversation_data)
            .map_err(|e| PlanAndExecuteError::SerializationError(e))?;
        
        self.store(
            MemoryEntryType::ConversationHistory,
            data,
            vec![
                format!("task:{}", task_id),
                "conversation".to_string(),
                "plan_execute".to_string(),
            ],
            Priority::High, // 对话历史高优先级保持
            Some(Duration::from_secs(24 * 60 * 60)), // 保持24小时
        ).await
    }

    /// 获取任务的完整对话历史
    pub async fn get_conversation_history(
        &self,
        task_id: &str,
    ) -> Result<Option<ConversationHistoryData>, PlanAndExecuteError> {
        let query = MemoryQuery {
            entry_types: Some(vec![MemoryEntryType::ConversationHistory]),
            tags: Some(vec![format!("task:{}", task_id)]),
            time_range: None,
            priority: None,
            limit: Some(1),
            sort_by: SortBy::LastAccessed,
        };
        
        let results = self.query(query).await?;
        if let Some(entry) = results.first() {
            let conversation_data: ConversationHistoryData = serde_json::from_value(entry.data.clone())
                .map_err(|e| PlanAndExecuteError::SerializationError(e))?;
            Ok(Some(conversation_data))
        } else {
            Ok(None)
        }
    }

    /// 更新对话历史（追加新的交互记录）
    pub async fn append_conversation_record(
        &self,
        task_id: &str,
        record: ConversationRecord,
    ) -> Result<(), PlanAndExecuteError> {
        let mut conversation_data = self.get_conversation_history(task_id).await?
            .unwrap_or_else(|| ConversationHistoryData::new(task_id.to_string()));
        
        conversation_data.records.push(record);
        conversation_data.updated_at = SystemTime::now();
        
        self.store_conversation_history(task_id, conversation_data).await?;
        Ok(())
    }

    /// 存储计划历史
    pub async fn store_plan_history(
        &self,
        task_id: &str,
        plan_data: PlanHistoryData,
    ) -> Result<String, PlanAndExecuteError> {
        let data = serde_json::to_value(&plan_data)
            .map_err(|e| PlanAndExecuteError::SerializationError(e))?;
        
        self.store(
            MemoryEntryType::PlanHistory,
            data,
            vec![
                format!("task:{}", task_id),
                "plan_history".to_string(),
                "plan_execute".to_string(),
            ],
            Priority::High,
            Some(Duration::from_secs(24 * 60 * 60)),
        ).await
    }

    /// 获取计划历史
    pub async fn get_plan_history(
        &self,
        task_id: &str,
    ) -> Result<Option<PlanHistoryData>, PlanAndExecuteError> {
        let query = MemoryQuery {
            entry_types: Some(vec![MemoryEntryType::PlanHistory]),
            tags: Some(vec![format!("task:{}", task_id)]),
            time_range: None,
            priority: None,
            limit: Some(1),
            sort_by: SortBy::LastAccessed,
        };
        
        let results = self.query(query).await?;
        if let Some(entry) = results.first() {
            let plan_data: PlanHistoryData = serde_json::from_value(entry.data.clone())
                .map_err(|e| PlanAndExecuteError::SerializationError(e))?;
            Ok(Some(plan_data))
        } else {
            Ok(None)
        }
    }

    /// 存储任务完整状态（包含所有上下文）
    pub async fn store_task_full_state(
        &self,
        task_id: &str,
        full_state: TaskFullStateData,
    ) -> Result<String, PlanAndExecuteError> {
        let data = serde_json::to_value(&full_state)
            .map_err(|e| PlanAndExecuteError::SerializationError(e))?;
        
        self.store(
            MemoryEntryType::TaskFullState,
            data,
            vec![
                format!("task:{}", task_id),
                "full_state".to_string(),
                "plan_execute".to_string(),
            ],
            Priority::Critical, // 最高优先级
            Some(Duration::from_secs(7 * 24 * 60 * 60)), // 保持7天
        ).await
    }

    /// 获取任务完整状态
    pub async fn get_task_full_state(
        &self,
        task_id: &str,
    ) -> Result<Option<TaskFullStateData>, PlanAndExecuteError> {
        let query = MemoryQuery {
            entry_types: Some(vec![MemoryEntryType::TaskFullState]),
            tags: Some(vec![format!("task:{}", task_id)]),
            time_range: None,
            priority: None,
            limit: Some(1),
            sort_by: SortBy::LastAccessed,
        };
        
        let results = self.query(query).await?;
        if let Some(entry) = results.first() {
            let full_state: TaskFullStateData = serde_json::from_value(entry.data.clone())
                .map_err(|e| PlanAndExecuteError::SerializationError(e))?;
            Ok(Some(full_state))
        } else {
            Ok(None)
        }
    }

    /// 构建Plan-and-Execute的完整上下文
    pub async fn build_plan_execute_context(
        &self,
        task_id: &str,
    ) -> Result<PlanExecuteContext, PlanAndExecuteError> {
        let conversation_history = self.get_conversation_history(task_id).await?;
        let plan_history = self.get_plan_history(task_id).await?;
        let full_state = self.get_task_full_state(task_id).await?;
        
        Ok(PlanExecuteContext {
            task_id: task_id.to_string(),
            conversation_history,
            plan_history,
            full_state,
            context_build_time: SystemTime::now(),
        })
    }

    // ===== 执行结果自动保存和摘要生成 =====

    /// 自动保存执行结果到内存（供Replanner使用）
    pub async fn save_execution_result(
        &self,
        task_id: &str,
        plan: &ExecutionPlan,
        result: &crate::engines::plan_and_execute::executor::ExecutionResult,
    ) -> Result<String, PlanAndExecuteError> {
        // 生成执行摘要
        let summary = self.generate_execution_summary(result).await;
        
        let execution_record = serde_json::json!({
            "task_id": task_id,
            "plan_id": plan.id,
            "plan_name": plan.name,
            "status": format!("{:?}", result.status),
            "completed_steps": result.completed_steps,
            "failed_steps": result.failed_steps,
            "skipped_steps": result.skipped_steps,
            "metrics": {
                "total_duration_ms": result.metrics.total_duration_ms,
                "successful_steps": result.metrics.successful_steps,
                "failed_steps": result.metrics.failed_steps,
                "total_retries": result.metrics.total_retries,
                "avg_step_duration_ms": result.metrics.avg_step_duration_ms,
                "peak_concurrency": result.metrics.peak_concurrency,
            },
            "summary": summary,
            "timestamp": SystemTime::now(),
        });
        
        self.store(
            MemoryEntryType::ExecutionState,
            execution_record,
            vec![
                format!("task:{}", task_id),
                format!("plan:{}", plan.id),
                "execution_result".to_string(),
            ],
            Priority::High,
            Some(Duration::from_secs(24 * 60 * 60)),
        ).await
    }

    /// 生成执行摘要（供AI理解）
    async fn generate_execution_summary(
        &self,
        result: &crate::engines::plan_and_execute::executor::ExecutionResult,
    ) -> ExecutionSummary {
        let total_steps = result.completed_steps.len() + result.failed_steps.len() + result.skipped_steps.len();
        let success_rate = if total_steps > 0 {
            result.completed_steps.len() as f64 / total_steps as f64 * 100.0
        } else {
            0.0
        };

        // 分析失败原因
        let mut failure_categories: HashMap<String, u32> = HashMap::new();
        for error in &result.errors {
            let category = match error.error_type {
                crate::engines::types::ErrorType::Tool => "tool_error",
                crate::engines::types::ErrorType::Network => "network_error",
                crate::engines::types::ErrorType::Timeout => "timeout_error",
                crate::engines::types::ErrorType::Configuration => "config_error",
                crate::engines::types::ErrorType::System => "system_error",
                crate::engines::types::ErrorType::Authentication => "auth_error",
                crate::engines::types::ErrorType::Permission => "permission_error",
                crate::engines::types::ErrorType::User => "user_error",
                crate::engines::types::ErrorType::Unknown => "unknown_error",
            };
            *failure_categories.entry(category.to_string()).or_insert(0) += 1;
        }

        // 识别关键问题
        let mut key_issues = Vec::new();
        if result.metrics.total_retries > 5 {
            key_issues.push("高重试次数，可能存在稳定性问题".to_string());
        }
        if result.metrics.avg_step_duration_ms > 30000 {
            key_issues.push("平均步骤执行时间过长".to_string());
        }
        if !result.failed_steps.is_empty() && result.completed_steps.is_empty() {
            key_issues.push("所有步骤都失败，需要完全重新规划".to_string());
        }

        // 生成改进建议
        let mut suggestions = Vec::new();
        if failure_categories.contains_key("timeout_error") {
            suggestions.push("考虑增加超时时间或优化工具性能".to_string());
        }
        if failure_categories.contains_key("network_error") {
            suggestions.push("检查网络连接或添加重试机制".to_string());
        }
        if result.skipped_steps.len() > 2 {
            suggestions.push("多个步骤被跳过，建议简化执行计划".to_string());
        }

        ExecutionSummary {
            overall_status: format!("{:?}", result.status),
            success_rate,
            total_steps,
            completed_count: result.completed_steps.len(),
            failed_count: result.failed_steps.len(),
            skipped_count: result.skipped_steps.len(),
            total_duration_ms: result.metrics.total_duration_ms,
            failure_categories,
            key_issues,
            suggestions,
            quality_score: result.enhanced_feedback.quality_assessment.overall_score as f64,
        }
    }

    /// 获取任务的执行历史摘要（供Replanner分析）
    pub async fn get_execution_history(
        &self,
        task_id: &str,
        limit: usize,
    ) -> Result<Vec<serde_json::Value>, PlanAndExecuteError> {
        let query = MemoryQuery {
            entry_types: Some(vec![MemoryEntryType::ExecutionState]),
            tags: Some(vec![format!("task:{}", task_id), "execution_result".to_string()]),
            time_range: None,
            priority: None,
            limit: Some(limit),
            sort_by: SortBy::CreatedAt,
        };
        
        let results = self.query(query).await?;
        Ok(results.into_iter().map(|e| e.data).collect())
    }

    /// 记录步骤执行开始
    pub async fn record_step_start(
        &self,
        task_id: &str,
        step_name: &str,
        step_type: &str,
    ) -> Result<(), PlanAndExecuteError> {
        let record = ConversationRecord {
            id: Uuid::new_v4().to_string(),
            record_type: ConversationRecordType::StepStart,
            content: format!("开始执行步骤: {} (类型: {})", step_name, step_type),
            step_id: Some(step_name.to_string()),
            plan_id: None,
            timestamp: SystemTime::now(),
            metadata: HashMap::new(),
        };
        
        self.append_conversation_record(task_id, record).await
    }

    /// 记录步骤执行完成
    pub async fn record_step_complete(
        &self,
        task_id: &str,
        step_name: &str,
        result: &serde_json::Value,
    ) -> Result<(), PlanAndExecuteError> {
        let mut metadata = HashMap::new();
        metadata.insert("result".to_string(), result.clone());
        
        let record = ConversationRecord {
            id: Uuid::new_v4().to_string(),
            record_type: ConversationRecordType::StepComplete,
            content: format!("步骤完成: {}", step_name),
            step_id: Some(step_name.to_string()),
            plan_id: None,
            timestamp: SystemTime::now(),
            metadata,
        };
        
        self.append_conversation_record(task_id, record).await
    }

    /// 记录步骤执行失败
    pub async fn record_step_failed(
        &self,
        task_id: &str,
        step_name: &str,
        error: &str,
    ) -> Result<(), PlanAndExecuteError> {
        let mut metadata = HashMap::new();
        metadata.insert("error".to_string(), serde_json::json!(error));
        
        let record = ConversationRecord {
            id: Uuid::new_v4().to_string(),
            record_type: ConversationRecordType::StepFailed,
            content: format!("步骤失败: {} - {}", step_name, error),
            step_id: Some(step_name.to_string()),
            plan_id: None,
            timestamp: SystemTime::now(),
            metadata,
        };
        
        self.append_conversation_record(task_id, record).await
    }

    /// 记录重新规划事件
    pub async fn record_replan_event(
        &self,
        task_id: &str,
        reason: &str,
        new_plan_id: &str,
    ) -> Result<(), PlanAndExecuteError> {
        let mut metadata = HashMap::new();
        metadata.insert("new_plan_id".to_string(), serde_json::json!(new_plan_id));
        metadata.insert("reason".to_string(), serde_json::json!(reason));
        
        let record = ConversationRecord {
            id: Uuid::new_v4().to_string(),
            record_type: ConversationRecordType::Replan,
            content: format!("触发重新规划: {}", reason),
            step_id: None,
            plan_id: Some(new_plan_id.to_string()),
            timestamp: SystemTime::now(),
            metadata,
        };
        
        self.append_conversation_record(task_id, record).await
    }
}

/// 执行摘要（供AI和Replanner使用）
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExecutionSummary {
    /// 总体状态
    pub overall_status: String,
    /// 成功率
    pub success_rate: f64,
    /// 总步骤数
    pub total_steps: usize,
    /// 完成数量
    pub completed_count: usize,
    /// 失败数量
    pub failed_count: usize,
    /// 跳过数量
    pub skipped_count: usize,
    /// 总执行时间(ms)
    pub total_duration_ms: u64,
    /// 失败分类
    pub failure_categories: HashMap<String, u32>,
    /// 关键问题
    pub key_issues: Vec<String>,
    /// 改进建议
    pub suggestions: Vec<String>,
    /// 质量评分
    pub quality_score: f64,
}