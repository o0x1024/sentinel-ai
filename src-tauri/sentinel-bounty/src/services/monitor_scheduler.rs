//! Asset Monitoring Scheduler
//!
//! Provides background monitoring service that periodically checks assets for changes
//! and automatically triggers workflows when changes are detected.

use std::collections::HashMap;
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::{RwLock, Mutex};
use tokio::time::interval;
use chrono::Utc;
use serde::{Deserialize, Serialize};
use std::pin::Pin;
use std::future::Future;
use tracing::{info, error};

use crate::services::change_monitor::{ChangeMonitor, ChangeMonitorConfig, AssetSnapshot};
use crate::models::ChangeEvent;

/// Callback for executing a monitor task
pub type TaskExecutor = Arc<dyn Fn(MonitorTask) -> Pin<Box<dyn Future<Output = Result<Vec<ChangeEvent>, String>> + Send>> + Send + Sync>;

/// Callback for change events
pub type EventCallback = Arc<dyn Fn(ChangeEvent) + Send + Sync>;

/// Monitoring task configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MonitorTask {
    /// Task ID
    pub id: String,
    /// Program ID to monitor
    pub program_id: String,
    /// Task name
    pub name: String,
    /// Check interval in seconds
    pub interval_secs: u64,
    /// Enabled status
    pub enabled: bool,
    /// Monitor configuration
    pub config: ChangeMonitorConfig,
    /// Last run timestamp
    pub last_run_at: Option<chrono::DateTime<chrono::Utc>>,
    /// Next scheduled run
    pub next_run_at: Option<chrono::DateTime<chrono::Utc>>,
    /// Total runs count
    pub run_count: u64,
    /// Events detected count
    pub events_detected: u64,
    /// Created timestamp
    pub created_at: chrono::DateTime<chrono::Utc>,
}

impl MonitorTask {
    pub fn new(program_id: String, name: String, interval_secs: u64) -> Self {
        let now = Utc::now();
        Self {
            id: uuid::Uuid::new_v4().to_string(),
            program_id,
            name,
            interval_secs,
            enabled: true,
            config: ChangeMonitorConfig::default(),
            last_run_at: None,
            next_run_at: Some(now + chrono::Duration::seconds(interval_secs as i64)),
            run_count: 0,
            events_detected: 0,
            created_at: now,
        }
    }

    /// Calculate next run time
    pub fn calculate_next_run(&mut self) {
        if self.enabled {
            self.next_run_at = Some(Utc::now() + chrono::Duration::seconds(self.interval_secs as i64));
        } else {
            self.next_run_at = None;
        }
    }
}

/// Monitor scheduler statistics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MonitorStats {
    pub total_tasks: usize,
    pub active_tasks: usize,
    pub total_runs: u64,
    pub total_events: u64,
    pub last_run_at: Option<chrono::DateTime<chrono::Utc>>,
    pub scheduler_uptime_secs: u64,
}

/// Asset monitoring scheduler
pub struct MonitorScheduler {
    /// Monitoring tasks
    tasks: Arc<RwLock<HashMap<String, MonitorTask>>>,
    /// Change monitors per program
    monitors: Arc<RwLock<HashMap<String, Arc<ChangeMonitor>>>>,
    /// Running state
    is_running: Arc<RwLock<bool>>,
    /// Scheduler start time
    start_time: Arc<RwLock<Option<chrono::DateTime<chrono::Utc>>>>,
    /// Event callback
    event_callback: Arc<Mutex<Option<EventCallback>>>,
    /// Task execution handler
    task_executor: Arc<Mutex<Option<TaskExecutor>>>,
}

impl Default for MonitorScheduler {
    fn default() -> Self {
        Self::new()
    }
}

impl MonitorScheduler {
    pub fn new() -> Self {
        Self {
            tasks: Arc::new(RwLock::new(HashMap::new())),
            monitors: Arc::new(RwLock::new(HashMap::new())),
            is_running: Arc::new(RwLock::new(false)),
            start_time: Arc::new(RwLock::new(None)),
            event_callback: Arc::new(Mutex::new(None)),
            task_executor: Arc::new(Mutex::new(None)),
        }
    }

    /// Check if scheduler is running
    pub async fn is_running(&self) -> bool {
        *self.is_running.read().await
    }

    /// Start the monitoring scheduler
    pub async fn start(&self) -> Result<(), String> {
        let mut running = self.is_running.write().await;
        if *running {
            return Err("Scheduler is already running".to_string());
        }
        *running = true;
        drop(running);

        let mut start_time = self.start_time.write().await;
        *start_time = Some(Utc::now());
        drop(start_time);

        info!("Starting asset monitoring scheduler");

        // Spawn background task
        let tasks = self.tasks.clone();
        let is_running = self.is_running.clone();
        let event_callback = self.event_callback.clone();
        let task_executor = self.task_executor.clone();

        tokio::spawn(async move {
            let mut tick_interval = interval(Duration::from_secs(10)); // Check every 10 seconds

            loop {
                tick_interval.tick().await;

                // Check if should stop
                if !*is_running.read().await {
                    info!("Monitoring scheduler stopped");
                    break;
                }

                // Get task IDs to check
                let task_ids: Vec<String> = {
                    let guard = tasks.read().await;
                    guard.keys().cloned().collect()
                };

                for task_id in task_ids {
                    // Check if should stop
                    if !*is_running.read().await {
                        break;
                    }

                    // 1. Check if task should run
                    let task_to_run = {
                        let guard = tasks.read().await;
                        guard.get(&task_id).and_then(|t| {
                            let should_run = t.enabled && t.next_run_at
                                .map(|next| Utc::now() >= next)
                                .unwrap_or(false);
                            if should_run { Some(t.clone()) } else { None }
                        })
                    };

                    let task = match task_to_run {
                        Some(t) => t,
                        None => continue,
                    };

                    info!("Running monitor task: {} ({})", task.name, task_id);

                    // 2. Get executor and monitor without holding task locks
                    let executor = {
                        let guard = task_executor.lock().await;
                        guard.clone()
                    };

                    // Run monitoring check
                    let mut detected_events = Vec::new();
                    
                    if let Some(exec) = executor {
                        info!("Executing task logic via registered executor...");
                        match exec(task.clone()).await {
                            Ok(events) => {
                                info!("Task execution successful, {} events generated", events.len());
                                detected_events = events;
                            }
                            Err(e) => {
                                error!("Task execution failed: {}", e);
                            }
                        }
                    } else {
                        info!("No task executor registered, skipping actual execution");
                    }
                    
                    // 3. Update task status with a brief write lock
                    {
                        let mut guard = tasks.write().await;
                        if let Some(t) = guard.get_mut(&task_id) {
                            t.last_run_at = Some(Utc::now());
                            t.run_count += 1;
                            t.calculate_next_run();
                            t.events_detected += detected_events.len() as u64;
                            
                            info!(
                                "Monitor task '{}' completed: {} events detected, next run at {:?}",
                                t.name,
                                detected_events.len(),
                                t.next_run_at
                            );
                        }
                    }

                    // 4. Trigger callbacks
                    if !detected_events.is_empty() {
                        let callback = {
                            let guard = event_callback.lock().await;
                            guard.clone()
                        };
                        
                        if let Some(cb) = callback {
                            for event in detected_events {
                                cb(event);
                            }
                        }
                    }
                }
            }
        });

        Ok(())
    }

    /// Stop the monitoring scheduler
    pub async fn stop(&self) -> Result<(), String> {
        let mut running = self.is_running.write().await;
        if !*running {
            return Err("Scheduler is not running".to_string());
        }
        *running = false;
        info!("Stopping asset monitoring scheduler");
        Ok(())
    }

    /// Add a monitoring task
    pub async fn add_task(&self, task: MonitorTask) -> Result<String, String> {
        let mut tasks = self.tasks.write().await;
        let task_id = task.id.clone();
        
        info!("Adding monitor task: {} ({})", task.name, task_id);
        tasks.insert(task_id.clone(), task);
        
        Ok(task_id)
    }

    /// Remove a monitoring task
    pub async fn remove_task(&self, task_id: &str) -> Result<(), String> {
        let mut tasks = self.tasks.write().await;
        tasks.remove(task_id)
            .ok_or_else(|| format!("Task not found: {}", task_id))?;
        info!("Removed monitor task: {}", task_id);
        Ok(())
    }

    /// Get a monitoring task
    pub async fn get_task(&self, task_id: &str) -> Option<MonitorTask> {
        let tasks = self.tasks.read().await;
        tasks.get(task_id).cloned()
    }

    /// List all monitoring tasks
    pub async fn list_tasks(&self) -> Vec<MonitorTask> {
        let tasks = self.tasks.read().await;
        tasks.values().cloned().collect()
    }

    /// Update task configuration
    pub async fn update_task(&self, task_id: &str, update_fn: impl FnOnce(&mut MonitorTask)) -> Result<(), String> {
        let mut tasks = self.tasks.write().await;
        let task = tasks.get_mut(task_id)
            .ok_or_else(|| format!("Task not found: {}", task_id))?;
        update_fn(task);
        Ok(())
    }

    /// Enable a task
    pub async fn enable_task(&self, task_id: &str) -> Result<(), String> {
        self.update_task(task_id, |task| {
            task.enabled = true;
            task.calculate_next_run();
        }).await
    }

    /// Disable a task
    pub async fn disable_task(&self, task_id: &str) -> Result<(), String> {
        self.update_task(task_id, |task| {
            task.enabled = false;
            task.next_run_at = None;
        }).await
    }

    /// Trigger a task immediately
    pub async fn trigger_task(&self, task_id: &str) -> Result<(), String> {
        self.update_task(task_id, |task| {
            task.next_run_at = Some(Utc::now());
        }).await
    }

    /// Get scheduler statistics
    pub async fn get_stats(&self) -> MonitorStats {
        let tasks = self.tasks.read().await;
        let start_time = self.start_time.read().await;

        let total_tasks = tasks.len();
        let active_tasks = tasks.values().filter(|t| t.enabled).count();
        let total_runs: u64 = tasks.values().map(|t| t.run_count).sum();
        let total_events: u64 = tasks.values().map(|t| t.events_detected).sum();
        let last_run_at = tasks.values()
            .filter_map(|t| t.last_run_at)
            .max();

        let uptime_secs = start_time
            .map(|st| (Utc::now() - st).num_seconds() as u64)
            .unwrap_or(0);

        MonitorStats {
            total_tasks,
            active_tasks,
            total_runs,
            total_events,
            last_run_at,
            scheduler_uptime_secs: uptime_secs,
        }
    }

    /// Set event callback
    pub async fn set_event_callback<F>(&self, callback: F)
    where
        F: Fn(ChangeEvent) + Send + Sync + 'static,
    {
        let mut cb = self.event_callback.lock().await;
        *cb = Some(Arc::new(callback));
    }

    /// Set task executor
    pub async fn set_task_executor<F>(&self, executor: F)
    where
        F: Fn(MonitorTask) -> Pin<Box<dyn Future<Output = Result<Vec<ChangeEvent>, String>> + Send>> + Send + Sync + 'static,
    {
        let mut exec = self.task_executor.lock().await;
        *exec = Some(Arc::new(executor));
    }

    /// Store asset snapshot for a program
    pub async fn store_snapshot(&self, program_id: &str, snapshot: AssetSnapshot) -> Result<(), String> {
        let monitors = self.monitors.read().await;
        if let Some(monitor) = monitors.get(program_id) {
            monitor.store_snapshot(snapshot).await;
            Ok(())
        } else {
            Err(format!("No monitor found for program: {}", program_id))
        }
    }

    /// Get or create monitor for a program
    pub async fn get_or_create_monitor(&self, program_id: &str, config: ChangeMonitorConfig) -> Arc<ChangeMonitor> {
        let mut monitors = self.monitors.write().await;
        monitors.entry(program_id.to_string())
            .or_insert_with(|| Arc::new(ChangeMonitor::with_config(config)))
            .clone()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_scheduler_lifecycle() {
        let scheduler = MonitorScheduler::new();
        
        assert!(!scheduler.is_running().await);
        
        scheduler.start().await.unwrap();
        assert!(scheduler.is_running().await);
        
        scheduler.stop().await.unwrap();
        assert!(!scheduler.is_running().await);
    }

    #[tokio::test]
    async fn test_task_management() {
        let scheduler = MonitorScheduler::new();
        
        let task = MonitorTask::new(
            "prog-1".to_string(),
            "Test Monitor".to_string(),
            3600,
        );
        let task_id = task.id.clone();
        
        scheduler.add_task(task).await.unwrap();
        
        let retrieved = scheduler.get_task(&task_id).await;
        assert!(retrieved.is_some());
        assert_eq!(retrieved.unwrap().name, "Test Monitor");
        
        scheduler.remove_task(&task_id).await.unwrap();
        assert!(scheduler.get_task(&task_id).await.is_none());
    }

    #[tokio::test]
    async fn test_task_enable_disable() {
        let scheduler = MonitorScheduler::new();
        
        let task = MonitorTask::new(
            "prog-1".to_string(),
            "Test Monitor".to_string(),
            3600,
        );
        let task_id = task.id.clone();
        
        scheduler.add_task(task).await.unwrap();
        
        scheduler.disable_task(&task_id).await.unwrap();
        let task = scheduler.get_task(&task_id).await.unwrap();
        assert!(!task.enabled);
        assert!(task.next_run_at.is_none());
        
        scheduler.enable_task(&task_id).await.unwrap();
        let task = scheduler.get_task(&task_id).await.unwrap();
        assert!(task.enabled);
        assert!(task.next_run_at.is_some());
    }
}
