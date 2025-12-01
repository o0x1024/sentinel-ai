//! 资源追踪器 - 跟踪和自动清理执行过程中的资源
//!
//! 解决资源泄露问题，提供自动清理机制

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use std::time::SystemTime;
use tokio::sync::RwLock;
use tracing::{info, warn, error};

/// 资源类型
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub enum ResourceType {
    /// 浏览器会话
    Browser,
    /// 代理服务器
    Proxy,
    /// 数据库连接
    Database,
    /// 文件句柄
    File,
    /// 后台进程
    Process,
    /// 网络连接
    Network,
    /// 临时文件
    TempFile,
    /// 自定义资源
    Custom(String),
}

/// 资源状态
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum ResourceState {
    /// 已创建/激活
    Active,
    /// 已释放
    Released,
    /// 释放失败
    ReleaseFailed,
}

/// 资源信息
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResourceInfo {
    /// 资源ID
    pub id: String,
    /// 资源类型
    pub resource_type: ResourceType,
    /// 资源状态
    pub state: ResourceState,
    /// 创建时间
    pub created_at: SystemTime,
    /// 释放时间
    pub released_at: Option<SystemTime>,
    /// 创建此资源的步骤
    pub created_by_step: Option<String>,
    /// 用于释放此资源的工具名称
    pub cleanup_tool: Option<String>,
    /// 清理工具参数
    pub cleanup_args: HashMap<String, serde_json::Value>,
    /// 附加元数据
    pub metadata: HashMap<String, serde_json::Value>,
}

/// 资源追踪器
#[derive(Debug)]
pub struct ResourceTracker {
    /// 活动资源
    resources: Arc<RwLock<HashMap<String, ResourceInfo>>>,
    /// 资源类型到清理工具的映射
    cleanup_tools: HashMap<ResourceType, CleanupConfig>,
}

/// 清理配置
#[derive(Debug, Clone)]
pub struct CleanupConfig {
    /// 清理工具名称
    pub tool_name: String,
    /// 默认参数
    pub default_args: HashMap<String, serde_json::Value>,
}

impl ResourceTracker {
    /// 创建新的资源追踪器
    pub fn new() -> Self {
        let mut cleanup_tools = HashMap::new();
        
        // 配置默认的资源清理工具
        cleanup_tools.insert(
            ResourceType::Browser,
            CleanupConfig {
                tool_name: "playwright_close".to_string(),
                default_args: HashMap::new(),
            },
        );
        
        cleanup_tools.insert(
            ResourceType::Proxy,
            CleanupConfig {
                tool_name: "stop_passive_scan".to_string(),
                default_args: HashMap::new(),
            },
        );

        Self {
            resources: Arc::new(RwLock::new(HashMap::new())),
            cleanup_tools,
        }
    }

    /// 注册新资源
    pub async fn register_resource(
        &self,
        resource_type: ResourceType,
        step_name: Option<String>,
        metadata: HashMap<String, serde_json::Value>,
    ) -> String {
        let id = uuid::Uuid::new_v4().to_string();
        
        let cleanup_config = self.cleanup_tools.get(&resource_type);
        
        let info = ResourceInfo {
            id: id.clone(),
            resource_type: resource_type.clone(),
            state: ResourceState::Active,
            created_at: SystemTime::now(),
            released_at: None,
            created_by_step: step_name.clone(),
            cleanup_tool: cleanup_config.map(|c| c.tool_name.clone()),
            cleanup_args: cleanup_config
                .map(|c| c.default_args.clone())
                .unwrap_or_default(),
            metadata,
        };

        let mut resources = self.resources.write().await;
        resources.insert(id.clone(), info);

        info!(
            "Resource registered: type={:?}, id={}, step={:?}",
            resource_type, id, step_name
        );
        
        id
    }

    /// 标记资源已释放
    pub async fn mark_released(&self, resource_id: &str) -> bool {
        let mut resources = self.resources.write().await;
        
        if let Some(info) = resources.get_mut(resource_id) {
            info.state = ResourceState::Released;
            info.released_at = Some(SystemTime::now());
            info!("Resource marked as released: id={}", resource_id);
            true
        } else {
            warn!("Resource not found for release: id={}", resource_id);
            false
        }
    }

    /// 根据工具名称标记相关资源已释放
    pub async fn mark_released_by_tool(&self, tool_name: &str) -> Vec<String> {
        let mut resources = self.resources.write().await;
        let mut released_ids = Vec::new();
        
        // 根据工具名称推断资源类型
        let resource_type = match tool_name {
            "playwright_close" => Some(ResourceType::Browser),
            "stop_passive_scan" => Some(ResourceType::Proxy),
            _ => None,
        };

        if let Some(rt) = resource_type {
            for (id, info) in resources.iter_mut() {
                if info.resource_type == rt && info.state == ResourceState::Active {
                    info.state = ResourceState::Released;
                    info.released_at = Some(SystemTime::now());
                    released_ids.push(id.clone());
                    info!("Resource released by tool {}: id={}", tool_name, id);
                }
            }
        }

        released_ids
    }

    /// 根据工具名称注册资源
    pub async fn register_by_tool(
        &self,
        tool_name: &str,
        step_name: Option<String>,
    ) -> Option<String> {
        let resource_type = match tool_name {
            "playwright_navigate" | "playwright_goto" => Some(ResourceType::Browser),
            "start_passive_scan" => Some(ResourceType::Proxy),
            _ => None,
        };

        if let Some(rt) = resource_type {
            Some(self.register_resource(rt, step_name, HashMap::new()).await)
        } else {
            None
        }
    }

    /// 获取所有活动资源
    pub async fn get_active_resources(&self) -> Vec<ResourceInfo> {
        let resources = self.resources.read().await;
        resources
            .values()
            .filter(|r| r.state == ResourceState::Active)
            .cloned()
            .collect()
    }

    /// 检查是否有资源泄露
    pub async fn has_resource_leak(&self) -> bool {
        !self.get_active_resources().await.is_empty()
    }

    /// 获取需要清理的资源及其清理工具
    pub async fn get_cleanup_tasks(&self) -> Vec<(ResourceInfo, String, HashMap<String, serde_json::Value>)> {
        let active = self.get_active_resources().await;
        let mut tasks = Vec::new();

        for resource in active {
            if let Some(tool) = &resource.cleanup_tool {
                tasks.push((
                    resource.clone(),
                    tool.clone(),
                    resource.cleanup_args.clone(),
                ));
            } else {
                warn!(
                    "No cleanup tool configured for resource type {:?}",
                    resource.resource_type
                );
            }
        }

        tasks
    }

    /// 生成资源清理报告
    pub async fn generate_cleanup_report(&self) -> ResourceCleanupReport {
        let resources = self.resources.read().await;
        
        let mut active_count = 0;
        let mut released_count = 0;
        let mut failed_count = 0;
        let mut by_type: HashMap<String, u32> = HashMap::new();
        let mut leaked_resources = Vec::new();

        for info in resources.values() {
            let type_key = format!("{:?}", info.resource_type);
            *by_type.entry(type_key).or_insert(0) += 1;

            match info.state {
                ResourceState::Active => {
                    active_count += 1;
                    leaked_resources.push(info.clone());
                }
                ResourceState::Released => released_count += 1,
                ResourceState::ReleaseFailed => failed_count += 1,
            }
        }

        ResourceCleanupReport {
            total_resources: resources.len(),
            active_count,
            released_count,
            failed_count,
            resources_by_type: by_type,
            leaked_resources,
            has_leaks: active_count > 0,
        }
    }

    /// 清理所有资源状态（用于新任务开始时）
    pub async fn clear_all(&self) {
        let mut resources = self.resources.write().await;
        resources.clear();
        info!("All resource tracking cleared");
    }
}

impl Default for ResourceTracker {
    fn default() -> Self {
        Self::new()
    }
}

/// 资源清理报告
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResourceCleanupReport {
    /// 总资源数
    pub total_resources: usize,
    /// 活动资源数
    pub active_count: usize,
    /// 已释放资源数
    pub released_count: usize,
    /// 释放失败资源数
    pub failed_count: usize,
    /// 按类型统计
    pub resources_by_type: HashMap<String, u32>,
    /// 泄露的资源
    pub leaked_resources: Vec<ResourceInfo>,
    /// 是否有泄露
    pub has_leaks: bool,
}

/// 资源追踪trait，用于步骤执行器
#[async_trait::async_trait]
pub trait ResourceAware {
    /// 在工具调用前注册资源
    async fn on_tool_start(&self, tool_name: &str, step_name: &str);
    
    /// 在工具调用后更新资源状态
    async fn on_tool_complete(&self, tool_name: &str, success: bool);
    
    /// 获取清理任务列表
    async fn get_pending_cleanups(&self) -> Vec<CleanupTask>;
}

/// 清理任务
#[derive(Debug, Clone)]
pub struct CleanupTask {
    /// 资源ID
    pub resource_id: String,
    /// 清理工具名称
    pub tool_name: String,
    /// 清理参数
    pub args: HashMap<String, serde_json::Value>,
    /// 优先级（数字越小优先级越高）
    pub priority: u32,
}

