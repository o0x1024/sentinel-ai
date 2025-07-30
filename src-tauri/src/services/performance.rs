use std::time::{Duration, Instant};
use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use serde::{Deserialize, Serialize};
use tokio::time::sleep;

/// 性能指标结构
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PerformanceMetrics {
    /// 内存使用情况
    pub memory_usage_mb: f64,
    /// CPU使用率
    pub cpu_usage_percent: f64,
    /// 活跃任务数
    pub active_tasks: usize,
    /// 平均响应时间
    pub avg_response_time_ms: f64,
    /// 错误率
    pub error_rate_percent: f64,
    /// 吞吐量 (请求/秒)
    pub throughput_rps: f64,
}

/// 性能监控器
#[derive(Debug, Clone)]
pub struct PerformanceMonitor {
    metrics: Arc<Mutex<PerformanceMetrics>>,
    timings: Arc<Mutex<HashMap<String, Vec<Duration>>>>,
    errors: Arc<Mutex<usize>>,
    requests: Arc<Mutex<usize>>,
    start_time: Instant,
}

impl Default for PerformanceMonitor {
    fn default() -> Self {
        Self::new()
    }
}

impl PerformanceMonitor {
    /// 创建新的性能监控器
    pub fn new() -> Self {
        Self {
            metrics: Arc::new(Mutex::new(PerformanceMetrics {
                memory_usage_mb: 0.0,
                cpu_usage_percent: 0.0,
                active_tasks: 0,
                avg_response_time_ms: 0.0,
                error_rate_percent: 0.0,
                throughput_rps: 0.0,
            })),
            timings: Arc::new(Mutex::new(HashMap::new())),
            errors: Arc::new(Mutex::new(0)),
            requests: Arc::new(Mutex::new(0)),
            start_time: Instant::now(),
        }
    }

    /// 开始后台监控任务
    pub async fn start_monitoring(&self) {
        let monitor = self.clone();
        tokio::spawn(async move {
            loop {
                monitor.update_system_metrics().await;
                sleep(Duration::from_secs(5)).await;
            }
        });
    }

    /// 更新系统性能指标
    async fn update_system_metrics(&self) {
        let mut metrics = self.metrics.lock().unwrap();
        
        // 更新内存使用情况
        metrics.memory_usage_mb = self.get_memory_usage();
        
        // 更新CPU使用率 (简化实现)
        metrics.cpu_usage_percent = self.get_cpu_usage();
        
        // 更新活跃任务数
        metrics.active_tasks = self.get_active_tasks();
        
        // 计算平均响应时间
        metrics.avg_response_time_ms = self.calculate_avg_response_time();
        
        // 计算错误率
        metrics.error_rate_percent = self.calculate_error_rate();
        
        // 计算吞吐量
        metrics.throughput_rps = self.calculate_throughput();
    }

    /// 获取内存使用情况（简化实现）
    fn get_memory_usage(&self) -> f64 {
        // 在实际应用中，可以使用系统调用获取真实内存使用
        // 这里返回一个模拟值
        50.0 + (rand::random::<f64>() * 20.0)
    }

    /// 获取CPU使用率（简化实现）
    fn get_cpu_usage(&self) -> f64 {
        // 在实际应用中，可以使用系统调用获取真实CPU使用率
        // 这里返回一个模拟值
        10.0 + (rand::random::<f64>() * 30.0)
    }

    /// 获取活跃任务数（简化实现）
    fn get_active_tasks(&self) -> usize {
        // 在实际应用中，可以跟踪真实的活跃任务
        // 注意: active_tasks_count方法在较新版本的tokio中已被移除
        // 这里返回一个模拟值
        5 + (rand::random::<usize>() % 10)
    }

    /// 记录操作时间
    pub fn record_timing(&self, operation: &str, duration: Duration) {
        let mut timings = self.timings.lock().unwrap();
        timings.entry(operation.to_string()).or_insert_with(Vec::new).push(duration);
        
        // 只保留最近100个记录
        let entry = timings.get_mut(operation).unwrap();
        if entry.len() > 100 {
            entry.remove(0);
        }
    }

    /// 计算平均响应时间
    fn calculate_avg_response_time(&self) -> f64 {
        let timings = self.timings.lock().unwrap();
        let mut total_duration = Duration::from_millis(0);
        let mut count = 0;

        for durations in timings.values() {
            for duration in durations {
                total_duration += *duration;
                count += 1;
            }
        }

        if count > 0 {
            total_duration.as_secs_f64() * 1000.0 / count as f64
        } else {
            0.0
        }
    }

    /// 记录请求
    pub fn record_request(&self) {
        let mut requests = self.requests.lock().unwrap();
        *requests += 1;
    }

    /// 记录错误
    pub fn record_error(&self) {
        let mut errors = self.errors.lock().unwrap();
        *errors += 1;
    }

    /// 计算错误率
    fn calculate_error_rate(&self) -> f64 {
        let errors = *self.errors.lock().unwrap();
        let requests = *self.requests.lock().unwrap();
        
        if requests > 0 {
            (errors as f64 / requests as f64) * 100.0
        } else {
            0.0
        }
    }

    /// 计算吞吐量
    fn calculate_throughput(&self) -> f64 {
        let requests = *self.requests.lock().unwrap();
        let elapsed = self.start_time.elapsed().as_secs_f64();
        
        if elapsed > 0.0 {
            requests as f64 / elapsed
        } else {
            0.0
        }
    }

    /// 获取当前性能指标
    pub fn get_metrics(&self) -> PerformanceMetrics {
        self.metrics.lock().unwrap().clone()
    }

    /// 生成性能报告
    pub fn generate_report(&self) -> String {
        let metrics = self.get_metrics();
        format!(
            r#"
=== Sentinel AI backend performance report ===

System resources:
- Memory usage: {:.2} MB
- CPU usage: {:.2}%
- Active tasks: {}

Request processing:
- Average response time: {:.2} ms
- Error rate: {:.2}%
- Throughput: {:.2} requests/second

Runtime: {:.2} seconds
            "#,
            metrics.memory_usage_mb,
            metrics.cpu_usage_percent,
            metrics.active_tasks,
            metrics.avg_response_time_ms,
            metrics.error_rate_percent,
            metrics.throughput_rps,
            self.start_time.elapsed().as_secs_f64()
        )
    }

    /// 重置统计数据
    pub fn reset_stats(&self) {
        let mut errors = self.errors.lock().unwrap();
        let mut requests = self.requests.lock().unwrap();
        let mut timings = self.timings.lock().unwrap();
        
        *errors = 0;
        *requests = 0;
        timings.clear();
    }
}

/// 性能监控装饰器宏
#[macro_export]
macro_rules! monitor_performance {
    ($monitor:expr, $operation:expr, $code:block) => {{
        let start = std::time::Instant::now();
        $monitor.record_request();
        
        let result = $code;
        
        match &result {
            Ok(_) => {
                let duration = start.elapsed();
                $monitor.record_timing($operation, duration);
            }
            Err(_) => {
                $monitor.record_error();
            }
        }
        
        result
    }};
}

/// 异步性能监控装饰器
pub async fn monitor_async<F, T, E>(
    monitor: &PerformanceMonitor,
    operation: &str,
    future: F,
) -> Result<T, E>
where
    F: std::future::Future<Output = Result<T, E>>,
{
    let start = Instant::now();
    monitor.record_request();
    
    let result = future.await;
    
    match &result {
        Ok(_) => {
            let duration = start.elapsed();
            monitor.record_timing(operation, duration);
        }
        Err(_) => {
            monitor.record_error();
        }
    }
    
    result
}

// 添加rand依赖来生成随机数
extern crate rand; 