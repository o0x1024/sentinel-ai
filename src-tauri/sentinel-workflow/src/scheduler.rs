//! 工作流定时调度器
//!
//! 支持间隔触发、每日触发、每周触发

use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::{Mutex, RwLock};
use tokio::task::JoinHandle;
use serde::{Deserialize, Serialize};
use chrono::{Local, Timelike, Datelike};

/// 调度配置
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScheduleConfig {
    pub trigger_type: String,           // interval, daily, weekly
    pub interval_seconds: Option<u64>,  // 间隔秒数
    pub hour: Option<u32>,              // 小时
    pub minute: Option<u32>,            // 分钟
    pub second: Option<u32>,            // 秒
    pub weekdays: Option<String>,       // 星期几，逗号分隔
}

/// 调度任务信息
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScheduleInfo {
    pub workflow_id: String,
    pub workflow_name: String,
    pub config: ScheduleConfig,
    pub is_running: bool,
    pub last_run: Option<String>,
    pub next_run: Option<String>,
    pub run_count: u64,
}

/// 内部任务状态
struct ScheduleTask {
    info: ScheduleInfo,
    cancel_token: tokio_util::sync::CancellationToken,
    handle: Option<JoinHandle<()>>,
}

/// 工作流调度器
pub struct WorkflowScheduler {
    tasks: Arc<RwLock<HashMap<String, ScheduleTask>>>,
    executor: Arc<dyn ScheduleExecutor + Send + Sync>,
}

/// 调度执行器 trait
#[async_trait::async_trait]
pub trait ScheduleExecutor: Send + Sync {
    async fn execute_workflow(&self, workflow_id: &str) -> Result<String, String>;
}

impl WorkflowScheduler {
    pub fn new(executor: Arc<dyn ScheduleExecutor + Send + Sync>) -> Self {
        Self {
            tasks: Arc::new(RwLock::new(HashMap::new())),
            executor,
        }
    }

    /// 启动定时任务
    pub async fn start_schedule(
        &self,
        workflow_id: String,
        workflow_name: String,
        config: ScheduleConfig,
    ) -> Result<(), String> {
        // 检查是否已存在
        {
            let tasks = self.tasks.read().await;
            if tasks.contains_key(&workflow_id) {
                return Err("Schedule already exists for this workflow".to_string());
            }
        }

        let cancel_token = tokio_util::sync::CancellationToken::new();
        let token_clone = cancel_token.clone();
        let executor = self.executor.clone();
        let wf_id = workflow_id.clone();
        let wf_name = workflow_name.clone();
        let cfg = config.clone();
        let tasks_ref = self.tasks.clone();

        // 计算下次运行时间
        let next_run = Self::calculate_next_run(&config);

        let info = ScheduleInfo {
            workflow_id: workflow_id.clone(),
            workflow_name: workflow_name.clone(),
            config: config.clone(),
            is_running: true,
            last_run: None,
            next_run: next_run.clone(),
            run_count: 0,
        };

        // 启动定时任务
        let handle = tokio::spawn(async move {
            Self::run_schedule_loop(
                wf_id,
                wf_name,
                cfg,
                token_clone,
                executor,
                tasks_ref,
            ).await;
        });

        // 保存任务
        let task = ScheduleTask {
            info,
            cancel_token,
            handle: Some(handle),
        };

        let mut tasks = self.tasks.write().await;
        tasks.insert(workflow_id, task);

        Ok(())
    }

    /// 停止定时任务
    pub async fn stop_schedule(&self, workflow_id: &str) -> Result<(), String> {
        let mut tasks = self.tasks.write().await;
        
        if let Some(task) = tasks.remove(workflow_id) {
            task.cancel_token.cancel();
            if let Some(handle) = task.handle {
                handle.abort();
            }
            tracing::info!("Stopped schedule for workflow: {}", workflow_id);
            Ok(())
        } else {
            Err("Schedule not found".to_string())
        }
    }

    /// 列出所有定时任务
    pub async fn list_schedules(&self) -> Vec<ScheduleInfo> {
        let tasks = self.tasks.read().await;
        tasks.values().map(|t| t.info.clone()).collect()
    }

    /// 获取单个任务信息
    pub async fn get_schedule(&self, workflow_id: &str) -> Option<ScheduleInfo> {
        let tasks = self.tasks.read().await;
        tasks.get(workflow_id).map(|t| t.info.clone())
    }

    /// 运行调度循环
    async fn run_schedule_loop(
        workflow_id: String,
        workflow_name: String,
        config: ScheduleConfig,
        cancel_token: tokio_util::sync::CancellationToken,
        executor: Arc<dyn ScheduleExecutor + Send + Sync>,
        tasks_ref: Arc<RwLock<HashMap<String, ScheduleTask>>>,
    ) {
        tracing::info!("[Scheduler] Started schedule loop for workflow: {} ({})", workflow_name, workflow_id);

        loop {
            // 计算等待时间
            let wait_duration = Self::calculate_wait_duration(&config);
            
            tracing::info!(
                "[Scheduler] Workflow {} next run in {} seconds",
                workflow_id,
                wait_duration.as_secs()
            );

            // 等待或取消
            tokio::select! {
                _ = tokio::time::sleep(wait_duration) => {
                    // 执行工作流
                    tracing::info!("[Scheduler] Triggering scheduled workflow: {}", workflow_name);
                    
                    match executor.execute_workflow(&workflow_id).await {
                        Ok(exec_id) => {
                            tracing::info!("[Scheduler] Scheduled workflow started: {} (exec_id: {})", workflow_name, exec_id);
                        }
                        Err(e) => {
                            tracing::error!("[Scheduler] Failed to start scheduled workflow {}: {}", workflow_name, e);
                        }
                    }

                    // 更新任务信息
                    let mut tasks = tasks_ref.write().await;
                    if let Some(task) = tasks.get_mut(&workflow_id) {
                        task.info.last_run = Some(Local::now().format("%Y-%m-%d %H:%M:%S").to_string());
                        task.info.run_count += 1;
                        task.info.next_run = Self::calculate_next_run(&config);
                    }
                }
                _ = cancel_token.cancelled() => {
                    tracing::info!("[Scheduler] Schedule cancelled for workflow: {}", workflow_id);
                    break;
                }
            }
        }
    }

    /// 计算等待时长
    fn calculate_wait_duration(config: &ScheduleConfig) -> std::time::Duration {
        match config.trigger_type.as_str() {
            "interval" => {
                let secs = config.interval_seconds.unwrap_or(60);
                std::time::Duration::from_secs(secs)
            }
            "daily" => {
                Self::duration_until_time(
                    config.hour.unwrap_or(9),
                    config.minute.unwrap_or(0),
                    config.second.unwrap_or(0),
                )
            }
            "weekly" => {
                let weekdays = Self::parse_weekdays(&config.weekdays.clone().unwrap_or_default());
                Self::duration_until_weekday_time(
                    &weekdays,
                    config.hour.unwrap_or(9),
                    config.minute.unwrap_or(0),
                    config.second.unwrap_or(0),
                )
            }
            _ => std::time::Duration::from_secs(60),
        }
    }

    /// 计算下次运行时间字符串
    fn calculate_next_run(config: &ScheduleConfig) -> Option<String> {
        let duration = Self::calculate_wait_duration(config);
        let next = Local::now() + chrono::Duration::from_std(duration).ok()?;
        Some(next.format("%Y-%m-%d %H:%M:%S").to_string())
    }

    /// 计算到指定时间的等待时长
    fn duration_until_time(hour: u32, minute: u32, second: u32) -> std::time::Duration {
        let now = Local::now();
        let today_target = now
            .with_hour(hour).unwrap()
            .with_minute(minute).unwrap()
            .with_second(second).unwrap();

        if today_target > now {
            (today_target - now).to_std().unwrap_or(std::time::Duration::from_secs(60))
        } else {
            // 明天同一时间
            let tomorrow_target = today_target + chrono::Duration::days(1);
            (tomorrow_target - now).to_std().unwrap_or(std::time::Duration::from_secs(60))
        }
    }

    /// 计算到指定星期和时间的等待时长
    fn duration_until_weekday_time(
        weekdays: &[u32],
        hour: u32,
        minute: u32,
        second: u32,
    ) -> std::time::Duration {
        if weekdays.is_empty() {
            return Self::duration_until_time(hour, minute, second);
        }

        let now = Local::now();
        let current_weekday = now.weekday().num_days_from_monday() + 1; // 1-7

        // 找到下一个匹配的星期几
        let mut min_days = 8u32;
        for &wd in weekdays {
            let days_until = if wd > current_weekday {
                wd - current_weekday
            } else if wd < current_weekday {
                7 - current_weekday + wd
            } else {
                // 同一天，检查时间是否已过
                let today_target = now
                    .with_hour(hour).unwrap()
                    .with_minute(minute).unwrap()
                    .with_second(second).unwrap();
                if today_target > now {
                    0
                } else {
                    7
                }
            };
            if days_until < min_days {
                min_days = days_until;
            }
        }

        let target = now + chrono::Duration::days(min_days as i64);
        let target = target
            .with_hour(hour).unwrap()
            .with_minute(minute).unwrap()
            .with_second(second).unwrap();

        if target > now {
            (target - now).to_std().unwrap_or(std::time::Duration::from_secs(60))
        } else {
            std::time::Duration::from_secs(60)
        }
    }

    /// 解析星期几字符串
    fn parse_weekdays(s: &str) -> Vec<u32> {
        s.split(',')
            .filter_map(|part| part.trim().parse::<u32>().ok())
            .filter(|&d| d >= 1 && d <= 7)
            .collect()
    }
}

