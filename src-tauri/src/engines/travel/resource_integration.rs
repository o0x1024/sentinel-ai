//! 资源追踪集成 - 自动追踪和清理执行过程中创建的资源
//!
//! 借鉴 Plan-and-Execute 的 ResourceTracker 设计

use super::types::*;
use crate::tools::{FrameworkToolAdapter, UnifiedToolCall};
use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use std::time::{Duration, SystemTime};
use tokio::sync::RwLock;

/// 资源追踪器
pub struct ResourceTracker {
    /// 活动资源
    resources: Arc<RwLock<HashMap<String, TrackedResource>>>,
    /// 资源类型到清理工具的映射
    cleanup_tools: HashMap<TrackedResourceType, CleanupConfig>,
    /// 是否启用自动清理
    auto_cleanup_enabled: bool,
}

/// 清理配置
#[derive(Debug, Clone)]
pub struct CleanupConfig {
    /// 清理工具名称
    pub tool_name: String,
    /// 默认参数
    pub default_args: HashMap<String, serde_json::Value>,
}

/// 资源清理报告
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResourceCleanupReport {
    /// 总资源数
    pub total_resources: usize,
    /// 活动资源数
    pub active_count: usize,
    /// 已清理资源数
    pub cleaned_count: usize,
    /// 清理失败数
    pub failed_count: usize,
    /// 按类型统计
    pub by_type: HashMap<String, u32>,
    /// 未清理的资源
    pub leaked_resources: Vec<TrackedResource>,
    /// 是否有泄露
    pub has_leaks: bool,
}

impl ResourceTracker {
    pub fn new() -> Self {
        let mut cleanup_tools = HashMap::new();

        // 配置默认的资源清理工具
        cleanup_tools.insert(
            TrackedResourceType::Browser,
            CleanupConfig {
                tool_name: "playwright_close".to_string(),
                default_args: HashMap::new(),
            },
        );

        cleanup_tools.insert(
            TrackedResourceType::Proxy,
            CleanupConfig {
                tool_name: "stop_passive_scan".to_string(),
                default_args: HashMap::new(),
            },
        );

        Self {
            resources: Arc::new(RwLock::new(HashMap::new())),
            cleanup_tools,
            auto_cleanup_enabled: true,
        }
    }

    /// 设置是否启用自动清理
    pub fn with_auto_cleanup(mut self, enabled: bool) -> Self {
        self.auto_cleanup_enabled = enabled;
        self
    }

    /// 根据工具名称判断是否会创建资源
    pub fn is_resource_creating_tool(tool_name: &str) -> Option<TrackedResourceType> {
        match tool_name {
            "playwright_navigate" | "playwright_goto" | "playwright_new_page" => {
                Some(TrackedResourceType::Browser)
            }
            "start_passive_scan" => Some(TrackedResourceType::Proxy),
            _ => None,
        }
    }

    /// 根据工具名称判断是否会释放资源
    pub fn is_resource_releasing_tool(tool_name: &str) -> Option<TrackedResourceType> {
        match tool_name {
            "playwright_close" => Some(TrackedResourceType::Browser),
            "stop_passive_scan" => Some(TrackedResourceType::Proxy),
            _ => None,
        }
    }

    /// 注册新资源
    pub async fn register_resource(
        &self,
        resource_type: TrackedResourceType,
        created_by: Option<String>,
    ) -> String {
        let id = uuid::Uuid::new_v4().to_string();

        let resource = TrackedResource {
            id: id.clone(),
            resource_type: resource_type.clone(),
            created_by,
            created_at: SystemTime::now(),
            cleaned: false,
        };

        let mut resources = self.resources.write().await;
        resources.insert(id.clone(), resource);

        log::info!(
            "ResourceTracker: Registered resource type={:?}, id={}",
            resource_type,
            id
        );

        id
    }

    /// 根据工具调用注册资源
    pub async fn on_tool_start(&self, tool_name: &str, step_name: &str) -> Option<String> {
        if let Some(resource_type) = Self::is_resource_creating_tool(tool_name) {
            Some(
                self.register_resource(resource_type, Some(step_name.to_string()))
                    .await,
            )
        } else {
            None
        }
    }

    /// 标记资源已清理
    pub async fn mark_cleaned(&self, resource_id: &str) -> bool {
        let mut resources = self.resources.write().await;

        if let Some(resource) = resources.get_mut(resource_id) {
            resource.cleaned = true;
            log::info!("ResourceTracker: Marked resource as cleaned: id={}", resource_id);
            true
        } else {
            log::warn!("ResourceTracker: Resource not found: id={}", resource_id);
            false
        }
    }

    /// 根据工具调用标记资源已清理
    pub async fn on_tool_complete(&self, tool_name: &str) -> Vec<String> {
        let mut cleaned_ids = Vec::new();

        if let Some(resource_type) = Self::is_resource_releasing_tool(tool_name) {
            let mut resources = self.resources.write().await;

            for (id, resource) in resources.iter_mut() {
                if resource.resource_type == resource_type && !resource.cleaned {
                    resource.cleaned = true;
                    cleaned_ids.push(id.clone());
                    log::info!(
                        "ResourceTracker: Resource released by tool {}: id={}",
                        tool_name,
                        id
                    );
                }
            }
        }

        cleaned_ids
    }

    /// 获取所有活动(未清理)资源
    pub async fn get_active_resources(&self) -> Vec<TrackedResource> {
        let resources = self.resources.read().await;
        resources
            .values()
            .filter(|r| !r.cleaned)
            .cloned()
            .collect()
    }

    /// 检查是否有资源泄露
    pub async fn has_resource_leak(&self) -> bool {
        !self.get_active_resources().await.is_empty()
    }

    /// 获取需要清理的任务列表
    pub async fn get_cleanup_tasks(&self) -> Vec<CleanupTask> {
        let active = self.get_active_resources().await;
        let mut tasks = Vec::new();

        for resource in active {
            if let Some(config) = self.cleanup_tools.get(&resource.resource_type) {
                tasks.push(CleanupTask {
                    resource_id: resource.id.clone(),
                    resource_type: resource.resource_type.clone(),
                    tool_name: config.tool_name.clone(),
                    args: config.default_args.clone(),
                });
            } else {
                log::warn!(
                    "ResourceTracker: No cleanup tool for resource type {:?}",
                    resource.resource_type
                );
            }
        }

        tasks
    }

    /// 执行清理任务
    pub async fn execute_cleanup(
        &self,
        tool_adapter: &Arc<dyn FrameworkToolAdapter>,
    ) -> Result<ResourceCleanupReport> {
        let tasks = self.get_cleanup_tasks().await;
        let total = tasks.len();
        let mut cleaned = 0;
        let mut failed = 0;

        log::info!("ResourceTracker: Starting cleanup of {} resources", total);

        for task in tasks {
            let unified_call = UnifiedToolCall {
                id: uuid::Uuid::new_v4().to_string(),
                tool_name: task.tool_name.clone(),
                parameters: task.args.clone(),
                timeout: Some(Duration::from_secs(30)),
                context: HashMap::new(),
                retry_count: 0,
            };

            match tool_adapter.execute_tool(unified_call).await {
                Ok(_) => {
                    self.mark_cleaned(&task.resource_id).await;
                    cleaned += 1;
                    log::info!(
                        "ResourceTracker: Cleaned resource {} with {}",
                        task.resource_id,
                        task.tool_name
                    );
                }
                Err(e) => {
                    failed += 1;
                    log::error!(
                        "ResourceTracker: Failed to clean resource {}: {}",
                        task.resource_id,
                        e
                    );
                }
            }
        }

        self.generate_report().await
    }

    /// 生成清理报告
    pub async fn generate_report(&self) -> Result<ResourceCleanupReport> {
        let resources = self.resources.read().await;

        let mut active_count = 0;
        let mut cleaned_count = 0;
        let mut by_type: HashMap<String, u32> = HashMap::new();
        let mut leaked_resources = Vec::new();

        for resource in resources.values() {
            let type_key = format!("{:?}", resource.resource_type);
            *by_type.entry(type_key).or_insert(0) += 1;

            if resource.cleaned {
                cleaned_count += 1;
            } else {
                active_count += 1;
                leaked_resources.push(resource.clone());
            }
        }

        Ok(ResourceCleanupReport {
            total_resources: resources.len(),
            active_count,
            cleaned_count,
            failed_count: 0,
            by_type,
            leaked_resources,
            has_leaks: active_count > 0,
        })
    }

    /// 清空所有追踪记录(用于新任务开始时)
    pub async fn clear_all(&self) {
        let mut resources = self.resources.write().await;
        resources.clear();
        log::info!("ResourceTracker: All tracking cleared");
    }
}

impl Default for ResourceTracker {
    fn default() -> Self {
        Self::new()
    }
}

/// 清理任务
#[derive(Debug, Clone)]
pub struct CleanupTask {
    pub resource_id: String,
    pub resource_type: TrackedResourceType,
    pub tool_name: String,
    pub args: HashMap<String, serde_json::Value>,
}

/// 资源感知 trait
#[async_trait::async_trait]
pub trait ResourceAware {
    /// 在工具调用前注册资源
    async fn before_tool_call(&self, tool_name: &str, step_name: &str);

    /// 在工具调用后更新资源状态
    async fn after_tool_call(&self, tool_name: &str, success: bool);

    /// 获取待清理任务列表
    async fn get_pending_cleanups(&self) -> Vec<CleanupTask>;
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_resource_tracking() {
        let tracker = ResourceTracker::new();

        // 注册资源
        let id = tracker
            .register_resource(TrackedResourceType::Browser, Some("step1".to_string()))
            .await;

        // 检查活动资源
        let active = tracker.get_active_resources().await;
        assert_eq!(active.len(), 1);
        assert!(!active[0].cleaned);

        // 标记清理
        tracker.mark_cleaned(&id).await;

        // 检查无泄露
        assert!(!tracker.has_resource_leak().await);
    }

    #[tokio::test]
    async fn test_tool_based_tracking() {
        let tracker = ResourceTracker::new();

        // 模拟 playwright_navigate 调用
        let id = tracker.on_tool_start("playwright_navigate", "step1").await;
        assert!(id.is_some());

        // 检查有活动资源
        assert!(tracker.has_resource_leak().await);

        // 模拟 playwright_close 调用
        tracker.on_tool_complete("playwright_close").await;

        // 检查无泄露
        assert!(!tracker.has_resource_leak().await);
    }
}

