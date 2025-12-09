//! 流式输出优化器
//!
//! 优化策略：
//! 1. 内容缓冲：累积小片段，批量发送减少 IPC 开销
//! 2. 去抖动：Todos 更新去抖动，避免高频更新
//! 3. 智能刷新：根据内容类型决定刷新时机

use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::sync::Mutex;

/// 流式内容缓冲器配置
#[derive(Clone)]
pub struct StreamBufferConfig {
    /// 最小缓冲大小（字符数）
    pub min_buffer_size: usize,
    /// 最大缓冲大小（字符数）
    pub max_buffer_size: usize,
    /// 刷新间隔（毫秒）
    pub flush_interval_ms: u64,
    /// 强制刷新的关键字符（如换行、句号）
    pub flush_on_chars: Vec<char>,
}

impl Default for StreamBufferConfig {
    fn default() -> Self {
        Self {
            min_buffer_size: 10,
            max_buffer_size: 100,
            flush_interval_ms: 50,
            flush_on_chars: vec!['\n', '。', '.', '！', '!', '？', '?'],
        }
    }
}

/// 流式内容缓冲器
pub struct StreamBuffer {
    config: StreamBufferConfig,
    buffer: Arc<Mutex<String>>,
    last_flush: Arc<Mutex<Instant>>,
    flushed_count: AtomicU64,
}

impl StreamBuffer {
    pub fn new(config: StreamBufferConfig) -> Self {
        Self {
            config,
            buffer: Arc::new(Mutex::new(String::new())),
            last_flush: Arc::new(Mutex::new(Instant::now())),
            flushed_count: AtomicU64::new(0),
        }
    }

    /// 添加内容到缓冲区，返回是否应该刷新
    pub async fn push(&self, content: &str) -> Option<String> {
        let mut buffer = self.buffer.lock().await;
        buffer.push_str(content);

        // 检查是否应该刷新
        if self.should_flush(&buffer).await {
            let flushed = std::mem::take(&mut *buffer);
            self.flushed_count.fetch_add(1, Ordering::Relaxed);
            *self.last_flush.lock().await = Instant::now();
            Some(flushed)
        } else {
            None
        }
    }

    /// 强制刷新缓冲区
    pub async fn flush(&self) -> Option<String> {
        let mut buffer = self.buffer.lock().await;
        if buffer.is_empty() {
            None
        } else {
            let flushed = std::mem::take(&mut *buffer);
            self.flushed_count.fetch_add(1, Ordering::Relaxed);
            *self.last_flush.lock().await = Instant::now();
            Some(flushed)
        }
    }

    /// 检查是否应该刷新
    async fn should_flush(&self, buffer: &str) -> bool {
        // 1. 超过最大缓冲大小
        if buffer.len() >= self.config.max_buffer_size {
            return true;
        }

        // 2. 包含强制刷新字符且超过最小大小
        if buffer.len() >= self.config.min_buffer_size {
            if let Some(last_char) = buffer.chars().last() {
                if self.config.flush_on_chars.contains(&last_char) {
                    return true;
                }
            }
        }

        // 3. 超过刷新间隔
        let last_flush = *self.last_flush.lock().await;
        if last_flush.elapsed() >= Duration::from_millis(self.config.flush_interval_ms) {
            return buffer.len() >= self.config.min_buffer_size;
        }

        false
    }

    /// 获取统计信息
    pub fn stats(&self) -> StreamBufferStats {
        StreamBufferStats {
            flush_count: self.flushed_count.load(Ordering::Relaxed),
        }
    }
}

/// 缓冲区统计
#[derive(Debug, Clone)]
pub struct StreamBufferStats {
    pub flush_count: u64,
}

/// 去抖动器配置
#[derive(Clone)]
pub struct DebounceConfig {
    /// 去抖动延迟（毫秒）
    pub delay_ms: u64,
    /// 最大等待时间（毫秒），超过后强制执行
    pub max_wait_ms: u64,
}

impl Default for DebounceConfig {
    fn default() -> Self {
        Self {
            delay_ms: 100,
            max_wait_ms: 500,
        }
    }
}

/// 去抖动器
pub struct Debouncer {
    config: DebounceConfig,
    last_call: Arc<Mutex<Instant>>,
    first_pending: Arc<Mutex<Option<Instant>>>,
    pending: AtomicBool,
}

impl Debouncer {
    pub fn new(config: DebounceConfig) -> Self {
        // 初始化 last_call 为过去的时间，确保第一次调用立即执行
        let past = Instant::now() - Duration::from_secs(10);
        Self {
            config,
            last_call: Arc::new(Mutex::new(past)),
            first_pending: Arc::new(Mutex::new(None)),
            pending: AtomicBool::new(false),
        }
    }

    /// 检查是否应该执行（去抖动后）
    pub async fn should_execute(&self) -> bool {
        let now = Instant::now();
        let mut last_call = self.last_call.lock().await;
        let mut first_pending = self.first_pending.lock().await;

        // 如果超过最大等待时间，强制执行
        if let Some(first) = *first_pending {
            if now.duration_since(first) >= Duration::from_millis(self.config.max_wait_ms) {
                *last_call = now;
                *first_pending = None;
                self.pending.store(false, Ordering::Relaxed);
                return true;
            }
        }

        // 检查去抖动延迟
        if now.duration_since(*last_call) >= Duration::from_millis(self.config.delay_ms) {
            *last_call = now;
            *first_pending = None;
            self.pending.store(false, Ordering::Relaxed);
            return true;
        }

        // 标记为待处理
        if first_pending.is_none() {
            *first_pending = Some(now);
        }
        self.pending.store(true, Ordering::Relaxed);
        false
    }

    /// 是否有待处理的调用
    pub fn has_pending(&self) -> bool {
        self.pending.load(Ordering::Relaxed)
    }

    /// 强制重置并执行
    pub async fn force_execute(&self) {
        let mut last_call = self.last_call.lock().await;
        let mut first_pending = self.first_pending.lock().await;
        *last_call = Instant::now();
        *first_pending = None;
        self.pending.store(false, Ordering::Relaxed);
    }
}

/// 批量更新收集器
pub struct BatchCollector<T> {
    items: Arc<Mutex<Vec<T>>>,
    max_batch_size: usize,
    debounce_delay_ms: u64,
    last_add: Arc<Mutex<Instant>>,
}

impl<T: Clone + Send> BatchCollector<T> {
    pub fn new(max_batch_size: usize, debounce_config: DebounceConfig) -> Self {
        Self {
            items: Arc::new(Mutex::new(Vec::new())),
            max_batch_size,
            debounce_delay_ms: debounce_config.delay_ms,
            last_add: Arc::new(Mutex::new(Instant::now())),
        }
    }

    /// 添加项目，返回是否应该刷新批次
    pub async fn add(&self, item: T) -> Option<Vec<T>> {
        let mut items = self.items.lock().await;
        items.push(item);
        *self.last_add.lock().await = Instant::now();

        // 仅在超过最大批次大小时刷新
        if items.len() >= self.max_batch_size {
            let batch = std::mem::take(&mut *items);
            return Some(batch);
        }

        None
    }

    /// 检查是否应该刷新（基于时间）
    pub async fn should_flush(&self) -> bool {
        let items = self.items.lock().await;
        if items.is_empty() {
            return false;
        }
        let last_add = *self.last_add.lock().await;
        last_add.elapsed() >= Duration::from_millis(self.debounce_delay_ms)
    }

    /// 尝试刷新（如果满足条件）
    pub async fn try_flush(&self) -> Option<Vec<T>> {
        if self.should_flush().await {
            Some(self.flush().await)
        } else {
            None
        }
    }

    /// 强制刷新所有待处理项目
    pub async fn flush(&self) -> Vec<T> {
        let mut items = self.items.lock().await;
        std::mem::take(&mut *items)
    }

    /// 是否有待处理项目
    pub async fn has_pending(&self) -> bool {
        !self.items.lock().await.is_empty()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_stream_buffer_basic() {
        let buffer = StreamBuffer::new(StreamBufferConfig {
            min_buffer_size: 3,  // 降低最小缓冲大小
            max_buffer_size: 20,
            flush_interval_ms: 100,
            flush_on_chars: vec!['\n'],
        });

        // 小片段不会立即刷新
        assert!(buffer.push("Hi").await.is_none());

        // 添加换行触发刷新（长度 3 >= min_buffer_size）
        let result = buffer.push("\n").await;
        assert!(result.is_some());
        assert_eq!(result.unwrap(), "Hi\n");
    }

    #[tokio::test]
    async fn test_stream_buffer_max_size() {
        let buffer = StreamBuffer::new(StreamBufferConfig {
            min_buffer_size: 5,
            max_buffer_size: 10,
            flush_interval_ms: 1000,
            flush_on_chars: vec![],
        });

        // 不断添加直到超过最大大小
        buffer.push("12345").await;
        let result = buffer.push("67890").await;
        assert!(result.is_some());
        assert_eq!(result.unwrap(), "1234567890");
    }

    #[tokio::test]
    async fn test_stream_buffer_flush() {
        let buffer = StreamBuffer::new(StreamBufferConfig::default());

        buffer.push("test").await;
        let result = buffer.flush().await;
        assert!(result.is_some());
        assert_eq!(result.unwrap(), "test");

        // 再次刷新应该返回 None
        assert!(buffer.flush().await.is_none());
    }

    #[tokio::test]
    async fn test_debouncer_basic() {
        let debouncer = Debouncer::new(DebounceConfig {
            delay_ms: 10,
            max_wait_ms: 100,
        });

        // 第一次调用应该执行
        assert!(debouncer.should_execute().await);

        // 立即再次调用不应该执行
        assert!(!debouncer.should_execute().await);
        assert!(debouncer.has_pending());

        // 等待超过延迟后应该执行
        tokio::time::sleep(Duration::from_millis(15)).await;
        assert!(debouncer.should_execute().await);
    }

    #[tokio::test]
    async fn test_batch_collector() {
        let collector: BatchCollector<i32> = BatchCollector::new(
            3,
            DebounceConfig {
                delay_ms: 1000,  // 较长延迟，确保不会因去抖动触发
                max_wait_ms: 5000,
            },
        );

        // 添加不足批次大小
        assert!(collector.add(1).await.is_none());
        assert!(collector.add(2).await.is_none());

        // 第三个触发批次（超过 max_batch_size）
        let batch = collector.add(3).await;
        assert!(batch.is_some());
        assert_eq!(batch.unwrap(), vec![1, 2, 3]);
    }

    #[tokio::test]
    async fn test_batch_collector_flush() {
        let collector: BatchCollector<i32> = BatchCollector::new(
            10,
            DebounceConfig {
                delay_ms: 1000,  // 较长延迟
                max_wait_ms: 5000,
            },
        );

        // 快速添加，不触发去抖动
        collector.add(1).await;
        collector.add(2).await;

        // 强制刷新
        let batch = collector.flush().await;
        assert_eq!(batch, vec![1, 2]);

        // 再次刷新应该为空
        assert!(collector.flush().await.is_empty());
    }
}

