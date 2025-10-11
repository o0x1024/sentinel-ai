//! 任务队列管理模块
//! 
//! 负责智能调度器的任务队列管理，包括优先级调度、负载均衡等

use std::collections::{BinaryHeap, HashMap, VecDeque};
use std::cmp::Ordering;

use tokio::sync::RwLock;
use serde::{Deserialize, Serialize};
use chrono::{DateTime, Utc};

use anyhow::Result;
use log::{info, warn, debug};

/// 任务项
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TaskItem {
    /// 任务ID
    pub id: String,
    /// 用户输入
    pub user_input: String,
    /// 用户ID
    pub user_id: String,
    /// 任务优先级
    pub priority: TaskPriority,
    /// 预估执行时间（秒）
    pub estimated_duration: Option<f64>,
    /// 资源需求
    pub resource_requirements: ResourceRequirements,
    /// 创建时间
    pub created_at: DateTime<Utc>,
    /// 调度时间
    pub scheduled_at: Option<DateTime<Utc>>,
    /// 开始执行时间
    pub started_at: Option<DateTime<Utc>>,
    /// 任务状态
    pub status: TaskStatus,
    /// 重试次数
    pub retry_count: u32,
    /// 最大重试次数
    pub max_retries: u32,
    /// 任务元数据
    pub metadata: HashMap<String, serde_json::Value>,
}

/// 任务优先级
#[derive(Debug, Clone, Copy, Hash, Serialize, Deserialize, PartialEq, Eq, PartialOrd, Ord)]
pub enum TaskPriority {
    /// 低优先级
    Low = 1,
    /// 普通优先级
    Normal = 2,
    /// 高优先级
    High = 3,
    /// 紧急优先级
    Critical = 4,
}

/// 资源需求
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResourceRequirements {
    /// CPU需求 (0.0-1.0)
    pub cpu: f32,
    /// 内存需求 (MB)
    pub memory_mb: u32,
    /// 网络带宽 (Mbps)
    pub network_mbps: f32,
    /// 存储需求 (MB)
    pub storage_mb: u32,
    /// 并发槽位需求
    pub concurrent_slots: u32,
}

/// 任务状态
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum TaskStatus {
    /// 等待中
    Pending,
    /// 已调度
    Scheduled,
    /// 执行中
    Running,
    /// 已完成
    Completed,
    /// 失败
    Failed,
    /// 已取消
    Cancelled,
    /// 超时
    Timeout,
}

/// 优先级任务包装器（用于BinaryHeap）
#[derive(Debug, Clone)]
struct PriorityTaskWrapper {
    task: TaskItem,
    priority_score: i64,
}

impl PartialEq for PriorityTaskWrapper {
    fn eq(&self, other: &Self) -> bool {
        self.priority_score == other.priority_score
    }
}

impl Eq for PriorityTaskWrapper {}

impl PartialOrd for PriorityTaskWrapper {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for PriorityTaskWrapper {
    fn cmp(&self, other: &Self) -> Ordering {
        // 注意：BinaryHeap是最大堆，我们需要优先级高的排在前面
        self.priority_score.cmp(&other.priority_score)
    }
}

/// 队列统计信息
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QueueStatistics {
    /// 总任务数
    pub total_tasks: u32,
    /// 等待任务数
    pub pending_tasks: u32,
    /// 执行中任务数
    pub running_tasks: u32,
    /// 已完成任务数
    pub completed_tasks: u32,
    /// 失败任务数
    pub failed_tasks: u32,
    /// 平均等待时间（秒）
    pub average_wait_time: f64,
    /// 平均执行时间（秒）
    pub average_execution_time: f64,
    /// 队列吞吐量（任务/小时）
    pub throughput: f64,
    /// 按优先级分布
    pub priority_distribution: HashMap<String, u32>,
}

/// 智能任务队列
pub struct TaskQueue {
    /// 优先级队列
    priority_queue: RwLock<BinaryHeap<PriorityTaskWrapper>>,
    /// 运行中的任务
    running_tasks: RwLock<HashMap<String, TaskItem>>,
    /// 已完成的任务（最近100个）
    completed_tasks: RwLock<VecDeque<TaskItem>>,
    /// 队列配置
    config: QueueConfig,
    /// 统计信息
    statistics: RwLock<QueueStatistics>,
}

/// 队列配置
#[derive(Debug, Clone)]
pub struct QueueConfig {
    /// 最大队列长度
    pub max_queue_size: usize,
    /// 最大并发任务数
    pub max_concurrent_tasks: usize,
    /// 任务超时时间（秒）
    pub default_timeout: f64,
    /// 优先级权重
    pub priority_weights: HashMap<TaskPriority, f32>,
    /// 启用负载均衡
    pub enable_load_balancing: bool,
    /// 启用自适应调度
    pub enable_adaptive_scheduling: bool,
}

impl Default for QueueConfig {
    fn default() -> Self {
        let mut priority_weights = HashMap::new();
        priority_weights.insert(TaskPriority::Low, 0.25);
        priority_weights.insert(TaskPriority::Normal, 0.5);
        priority_weights.insert(TaskPriority::High, 0.75);
        priority_weights.insert(TaskPriority::Critical, 1.0);

        Self {
            max_queue_size: 1000,
            max_concurrent_tasks: 10,
            default_timeout: 1800, // 30分钟
            priority_weights,
            enable_load_balancing: true,
            enable_adaptive_scheduling: true,
        }
    }
}

impl TaskQueue {
    /// 创建新的任务队列
    pub fn new(config: Option<QueueConfig>) -> Self {
        Self {
            priority_queue: RwLock::new(BinaryHeap::new()),
            running_tasks: RwLock::new(HashMap::new()),
            completed_tasks: RwLock::new(VecDeque::new()),
            config: config.unwrap_or_default(),
            statistics: RwLock::new(QueueStatistics {
                total_tasks: 0,
                pending_tasks: 0,
                running_tasks: 0,
                completed_tasks: 0,
                failed_tasks: 0,
                average_wait_time: 0.0,
                average_execution_time: 0.0,
                throughput: 0.0,
                priority_distribution: HashMap::new(),
            }),
        }
    }

    /// 添加任务到队列
    pub async fn enqueue_task(&self, mut task: TaskItem) -> Result<()> {
        info!("Enqueuing task: {} with priority: {:?}", task.id, task.priority);

        // 检查队列大小限制
        {
            let queue = self.priority_queue.read().await;
            if queue.len() >= self.config.max_queue_size {
                return Err(anyhow::anyhow!("Queue is full, cannot enqueue more tasks"));
            }
        }

        // 设置任务状态和时间
        task.status = TaskStatus::Pending;
        task.created_at = Utc::now();

        // 计算优先级分数
        let priority_score = self.calculate_priority_score(&task);

        // 添加到优先级队列
        {
            let mut queue = self.priority_queue.write().await;
            queue.push(PriorityTaskWrapper {
                task: task.clone(),
                priority_score,
            });
        }

        // 更新统计信息
        self.update_statistics_on_enqueue(&task).await;

        debug!("Task {} enqueued successfully with priority score: {}", task.id, priority_score);
        Ok(())
    }

    /// 从队列中获取下一个任务
    pub async fn dequeue_task(&self) -> Option<TaskItem> {
        // 检查并发限制
        {
            let running = self.running_tasks.read().await;
            if running.len() >= self.config.max_concurrent_tasks {
                debug!("Max concurrent tasks reached, cannot dequeue");
                return None;
            }
        }

        // 从优先级队列获取任务
        let task_wrapper = {
            let mut queue = self.priority_queue.write().await;
            queue.pop()
        };

        if let Some(wrapper) = task_wrapper {
            let mut task = wrapper.task;
            task.status = TaskStatus::Scheduled;
            task.scheduled_at = Some(Utc::now());

            // 添加到运行中任务
            {
                let mut running = self.running_tasks.write().await;
                running.insert(task.id.clone(), task.clone());
            }

            info!("Dequeued task: {} for execution", task.id);
            Some(task)
        } else {
            None
        }
    }

    /// 标记任务开始执行
    pub async fn mark_task_started(&self, task_id: &str) -> Result<()> {
        let mut running = self.running_tasks.write().await;
        if let Some(task) = running.get_mut(task_id) {
            task.status = TaskStatus::Running;
            task.started_at = Some(Utc::now());
            info!("Task {} started execution", task_id);
            Ok(())
        } else {
            Err(anyhow::anyhow!("Task not found in running tasks: {}", task_id))
        }
    }

    /// 标记任务完成
    pub async fn mark_task_completed(&self, task_id: &str, _result: Option<serde_json::Value>) -> Result<()> {
        let task = {
            let mut running = self.running_tasks.write().await;
            running.remove(task_id)
        };

        if let Some(mut task) = task {
            task.status = TaskStatus::Completed;
            
            // 添加到已完成任务历史
            {
                let mut completed = self.completed_tasks.write().await;
                completed.push_back(task);
                
                // 保持最近100个任务
                if completed.len() > 100 {
                    completed.pop_front();
                }
            }

            // 更新统计信息
            self.update_statistics_on_completion(task_id, true).await;

            info!("Task {} completed successfully", task_id);
            Ok(())
        } else {
            Err(anyhow::anyhow!("Task not found: {}", task_id))
        }
    }

    /// 标记任务失败
    pub async fn mark_task_failed(&self, task_id: &str, error: &str) -> Result<()> {
        let task = {
            let mut running = self.running_tasks.write().await;
            running.remove(task_id)
        };

        if let Some(mut task) = task {
            task.status = TaskStatus::Failed;
            task.retry_count += 1;

            // 检查是否需要重试
            if task.retry_count < task.max_retries {
                warn!("Task {} failed, retrying (attempt {}/{})", task_id, task.retry_count, task.max_retries);
                
                // 重新加入队列
                let priority_score = self.calculate_priority_score(&task);
                {
                    let mut queue = self.priority_queue.write().await;
                    queue.push(PriorityTaskWrapper {
                        task,
                        priority_score,
                    });
                }
            } else {
                warn!("Task {} failed permanently after {} retries: {}", task_id, task.retry_count, error);
                
                // 添加到已完成任务历史
                {
                    let mut completed = self.completed_tasks.write().await;
                    completed.push_back(task);
                    
                    if completed.len() > 100 {
                        completed.pop_front();
                    }
                }

                // 更新统计信息
                self.update_statistics_on_completion(task_id, false).await;
            }

            Ok(())
        } else {
            Err(anyhow::anyhow!("Task not found: {}", task_id))
        }
    }

    /// 取消任务
    pub async fn cancel_task(&self, task_id: &str) -> Result<()> {
        // 首先尝试从运行中任务移除
        {
            let mut running = self.running_tasks.write().await;
            if let Some(mut task) = running.remove(task_id) {
                task.status = TaskStatus::Cancelled;
                info!("Cancelled running task: {}", task_id);
                return Ok(());
            }
        }

        // 如果不在运行中，尝试从队列中移除
        {
            let mut queue = self.priority_queue.write().await;
            let original_len = queue.len();
            
            // 重建堆，排除要取消的任务
            let mut temp_vec: Vec<_> = queue.drain().collect();
            temp_vec.retain(|wrapper| wrapper.task.id != task_id);
            
            for wrapper in temp_vec {
                queue.push(wrapper);
            }

            if queue.len() < original_len {
                info!("Cancelled queued task: {}", task_id);
                Ok(())
            } else {
                Err(anyhow::anyhow!("Task not found: {}", task_id))
            }
        }
    }

    /// 获取队列状态
    pub async fn get_queue_status(&self) -> Result<QueueStatistics> {
        let statistics = self.statistics.read().await;
        Ok(statistics.clone())
    }

    /// 获取任务详情
    pub async fn get_task_details(&self, task_id: &str) -> Option<TaskItem> {
        // 首先检查运行中任务
        {
            let running = self.running_tasks.read().await;
            if let Some(task) = running.get(task_id) {
                return Some(task.clone());
            }
        }

        // 然后检查已完成任务
        {
            let completed = self.completed_tasks.read().await;
            for task in completed.iter() {
                if task.id == task_id {
                    return Some(task.clone());
                }
            }
        }

        // 最后检查队列中的任务
        {
            let queue = self.priority_queue.read().await;
            for wrapper in queue.iter() {
                if wrapper.task.id == task_id {
                    return Some(wrapper.task.clone());
                }
            }
        }

        None
    }

    /// 计算任务优先级分数
    fn calculate_priority_score(&self, task: &TaskItem) -> i64 {
        let base_priority = task.priority as i64 * 1000;
        
        // 考虑任务创建时间（越早创建分数越高）
        let age_score = -(task.created_at.timestamp() % 1000);
        
        // 考虑重试次数（重试任务优先级稍低）
        let retry_penalty = -(task.retry_count as i64 * 10);
        
        base_priority + age_score + retry_penalty
    }

    /// 更新入队统计信息
    async fn update_statistics_on_enqueue(&self, task: &TaskItem) {
        let mut stats = self.statistics.write().await;
        stats.total_tasks += 1;
        stats.pending_tasks += 1;
        
        let priority_key = format!("{:?}", task.priority);
        *stats.priority_distribution.entry(priority_key).or_insert(0) += 1;
    }

    /// 更新完成统计信息
    async fn update_statistics_on_completion(&self, _task_id: &str, success: bool) {
        let mut stats = self.statistics.write().await;
        stats.running_tasks = stats.running_tasks.saturating_sub(1);
        
        if success {
            stats.completed_tasks += 1;
        } else {
            stats.failed_tasks += 1;
        }
    }

    /// 清理超时任务
    pub async fn cleanup_timeout_tasks(&self) -> u32 {
        let mut cleaned_count = 0;
        let timeout_duration = chrono::Duration::seconds(self.config.default_timeout as i64);
        let now = Utc::now();

        let timeout_task_ids: Vec<String> = {
            let running = self.running_tasks.read().await;
            running.iter()
                .filter_map(|(id, task)| {
                    if let Some(started_at) = task.started_at {
                        if now.signed_duration_since(started_at) > timeout_duration {
                            Some(id.clone())
                        } else {
                            None
                        }
                    } else {
                        None
                    }
                })
                .collect()
        };

        for task_id in timeout_task_ids {
            if let Err(e) = self.mark_task_failed(&task_id, "Task timeout").await {
                warn!("Failed to mark task as timeout: {}", e);
            } else {
                cleaned_count += 1;
            }
        }

        if cleaned_count > 0 {
            info!("Cleaned up {} timeout tasks", cleaned_count);
        }

        cleaned_count
    }
}
