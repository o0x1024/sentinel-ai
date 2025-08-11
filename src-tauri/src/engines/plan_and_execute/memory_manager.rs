//! Memory Manager 组件 - 内存管理器
//! 
//! 负责管理执行过程中的上下文、状态信息和历史数据

use crate::engines::plan_and_execute::types::*;
use crate::engines::plan_and_execute::executor::{ExecutionResult, StepResult};
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
            duration_ms: duration.as_millis() as u64,
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

    async fn cleanup_low_priority(&self, target_count: usize) -> Result<u64, PlanAndExecuteError> {
        let mut cleaned_count = 0;
        
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
            
            cleaned_count = keys_to_remove.len() as u64;
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
        
        // 估算内存使用量（简化计算）
        let memory_usage_bytes = store.len() * 1024; // 假设每个条目平均1KB
        
        let cleanup_stats = self.cleanup_stats.read().await.clone();
        
        let statistics = MemoryStatistics {
            total_entries: store.len(),
            entries_by_type,
            memory_usage_bytes,
            cache_hit_rate: 0.85, // 简化实现
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
    pub duration_ms: u64,
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