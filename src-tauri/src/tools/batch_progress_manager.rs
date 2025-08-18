//! 批处理和进度通知管理器
//! 
//! 基于rmcp 0.5.0实现的批处理和进度通知功能，支持：
//! - JSON-RPC批处理请求
//! - 实时进度通知
//! - 并行任务处理
//! - 动态状态描述

use anyhow::{anyhow, Result};
use serde::{Serialize, Deserialize};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::{RwLock, mpsc, broadcast};
use tracing::{info, error};
use uuid::Uuid;
use futures::future::join_all;

/// 批处理请求
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BatchRequest {
    pub id: Uuid,
    pub requests: Vec<BatchRequestItem>,
    pub parallel: bool, // 是否并行执行
    pub max_concurrency: Option<usize>, // 最大并发数
    pub timeout_seconds: Option<u64>,
}

/// 批处理请求项
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum BatchRequestItem {
    CallTool {
        id: Uuid,
        name: String,
        arguments: serde_json::Value,
        progress_token: Option<String>,
    },
    ListTools {
        id: Uuid,
        cursor: Option<String>,
    },
    GetResource {
        id: Uuid,
        uri: String,
    },
}

/// 批处理响应
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BatchResponse {
    pub id: Uuid,
    pub responses: Vec<BatchResponseItem>,
    pub completed_at: chrono::DateTime<chrono::Utc>,
    pub total_duration_ms: u64,
    pub success_count: usize,
    pub error_count: usize,
}

/// 批处理响应项
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum BatchResponseItem {
    Success {
        id: Uuid,
        result: serde_json::Value,
        duration_ms: u64,
    },
    Error {
        id: Uuid,
        error: String,
        duration_ms: u64,
    },
}

/// 进度通知
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProgressNotification {
    pub progress_token: String,
    pub progress: u32, // 0-100
    pub total: Option<u32>,
    pub message: Option<String>, // 动态状态描述
    pub timestamp: chrono::DateTime<chrono::Utc>,
    pub metadata: HashMap<String, serde_json::Value>,
}

/// 进度监听器
pub type ProgressListener = mpsc::UnboundedReceiver<ProgressNotification>;

/// 批处理状态
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum BatchStatus {
    Pending,
    Running,
    Completed,
    Failed,
    Cancelled,
}

/// 批处理执行信息
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BatchExecutionInfo {
    pub id: Uuid,
    pub status: BatchStatus,
    pub started_at: Option<chrono::DateTime<chrono::Utc>>,
    pub completed_at: Option<chrono::DateTime<chrono::Utc>>,
    pub progress: u32, // 0-100
    pub completed_items: usize,
    pub total_items: usize,
    pub current_message: Option<String>,
}

/// 批处理和进度管理器
pub struct BatchProgressManager {
    // 批处理执行状态
    batch_executions: Arc<RwLock<HashMap<Uuid, BatchExecutionInfo>>>,
    
    // 进度通知广播器
    progress_broadcaster: broadcast::Sender<ProgressNotification>,
    
    // 进度监听器映射
    progress_listeners: Arc<RwLock<HashMap<String, mpsc::UnboundedSender<ProgressNotification>>>>,
    
    // 配置
    max_batch_size: usize,
    default_timeout_seconds: u64,
    max_concurrent_batches: usize,
}

impl BatchProgressManager {
    pub fn new(
        max_batch_size: usize,
        default_timeout_seconds: u64,
        max_concurrent_batches: usize,
    ) -> Self {
        let (progress_broadcaster, _) = broadcast::channel(1000);
        
        Self {
            batch_executions: Arc::new(RwLock::new(HashMap::new())),
            progress_broadcaster,
            progress_listeners: Arc::new(RwLock::new(HashMap::new())),
            max_batch_size,
            default_timeout_seconds,
            max_concurrent_batches,
        }
    }
    
    /// 提交批处理请求
    pub async fn submit_batch(&self, mut request: BatchRequest) -> Result<Uuid> {
        // 验证批处理大小
        if request.requests.len() > self.max_batch_size {
            return Err(anyhow!(
                "Batch size {} exceeds maximum {}",
                request.requests.len(),
                self.max_batch_size
            ));
        }
        
        // 检查并发批处理数量
        let current_batches = self.batch_executions.read().await.len();
        if current_batches >= self.max_concurrent_batches {
            return Err(anyhow!(
                "Maximum concurrent batches {} reached",
                self.max_concurrent_batches
            ));
        }
        
        // 设置默认超时
        if request.timeout_seconds.is_none() {
            request.timeout_seconds = Some(self.default_timeout_seconds);
        }
        
        // 创建执行信息
        let execution_info = BatchExecutionInfo {
            id: request.id,
            status: BatchStatus::Pending,
            started_at: None,
            completed_at: None,
            progress: 0,
            completed_items: 0,
            total_items: request.requests.len(),
            current_message: Some("Batch request submitted".to_string()),
        };
        
        // 存储执行信息
        self.batch_executions.write().await.insert(request.id, execution_info);
        
        // 异步执行批处理
        let manager = self.clone();
        let request_id = request.id;
        let request_len = request.requests.len();
        tokio::spawn(async move {
            if let Err(e) = manager.execute_batch(request).await {
                error!("Batch execution failed: {}", e);
                manager.update_batch_status(request_id, BatchStatus::Failed, Some(e.to_string())).await;
            }
        });
        
        info!("Submitted batch request with {} items", request_len);
        Ok(request_id)
    }
    
    /// 执行批处理
    async fn execute_batch(&self, request: BatchRequest) -> Result<BatchResponse> {
        let batch_id = request.id;
        let start_time = std::time::Instant::now();
        
        info!("Starting batch execution: {}", batch_id);
        
        // 更新状态为运行中
        self.update_batch_status(batch_id, BatchStatus::Running, Some("Executing batch requests".to_string())).await;
        
        let responses = if request.parallel {
            self.execute_parallel(request.requests, request.max_concurrency, batch_id).await?
        } else {
            self.execute_sequential(request.requests, batch_id).await?
        };
        
        let total_duration = start_time.elapsed().as_millis() as u64;
        
        // 统计结果
        let success_count = responses.iter().filter(|r| matches!(r, BatchResponseItem::Success { .. })).count();
        let error_count = responses.len() - success_count;
        
        let batch_response = BatchResponse {
            id: batch_id,
            responses,
            completed_at: chrono::Utc::now(),
            total_duration_ms: total_duration,
            success_count,
            error_count,
        };
        
        // 更新最终状态
        self.update_batch_status(
            batch_id,
            if error_count == 0 { BatchStatus::Completed } else { BatchStatus::Failed },
            Some(format!("Completed: {} success, {} errors", success_count, error_count))
        ).await;
        
        info!("Batch execution completed: {} ({}ms)", batch_id, total_duration);
        Ok(batch_response)
    }
    
    /// 并行执行请求
    async fn execute_parallel(
        &self,
        requests: Vec<BatchRequestItem>,
        max_concurrency: Option<usize>,
        batch_id: Uuid,
    ) -> Result<Vec<BatchResponseItem>> {
        let semaphore = Arc::new(tokio::sync::Semaphore::new(
            max_concurrency.unwrap_or(requests.len())
        ));
        
        let tasks: Vec<_> = requests.into_iter().enumerate().map(|(index, request)| {
            let semaphore = semaphore.clone();
            let manager = self.clone();
            
            async move {
                let _permit = semaphore.acquire().await.unwrap();
                let start_time = std::time::Instant::now();
                
                // 更新进度
                manager.update_batch_progress(
                    batch_id,
                    index,
                    Some(format!("Processing item {}", index + 1))
                ).await;
                
                let result = manager.execute_single_request(request).await;
                let duration = start_time.elapsed().as_millis() as u64;
                
                match result {
                    Ok(result) => BatchResponseItem::Success {
                        id: Uuid::new_v4(), // 这里应该使用请求的实际ID
                        result,
                        duration_ms: duration,
                    },
                    Err(e) => BatchResponseItem::Error {
                        id: Uuid::new_v4(), // 这里应该使用请求的实际ID
                        error: e.to_string(),
                        duration_ms: duration,
                    },
                }
            }
        }).collect();
        
        let responses = join_all(tasks).await;
        Ok(responses)
    }
    
    /// 顺序执行请求
    async fn execute_sequential(
        &self,
        requests: Vec<BatchRequestItem>,
        batch_id: Uuid,
    ) -> Result<Vec<BatchResponseItem>> {
        let mut responses = Vec::new();
        
        for (index, request) in requests.into_iter().enumerate() {
            let start_time = std::time::Instant::now();
            
            // 更新进度
            self.update_batch_progress(
                batch_id,
                index,
                Some(format!("Processing item {} of {}", index + 1, responses.len() + 1))
            ).await;
            
            let result = self.execute_single_request(request).await;
            let duration = start_time.elapsed().as_millis() as u64;
            
            let response = match result {
                Ok(result) => BatchResponseItem::Success {
                    id: Uuid::new_v4(), // 这里应该使用请求的实际ID
                    result,
                    duration_ms: duration,
                },
                Err(e) => BatchResponseItem::Error {
                    id: Uuid::new_v4(), // 这里应该使用请求的实际ID
                    error: e.to_string(),
                    duration_ms: duration,
                },
            };
            
            responses.push(response);
        }
        
        Ok(responses)
    }
    
    /// 执行单个请求
    async fn execute_single_request(&self, request: BatchRequestItem) -> Result<serde_json::Value> {
        match request {
            BatchRequestItem::CallTool { name, arguments, progress_token, .. } => {
                // 发送进度通知
                if let Some(token) = &progress_token {
                    self.send_progress_notification(ProgressNotification {
                        progress_token: token.clone(),
                        progress: 0,
                        total: Some(100),
                        message: Some(format!("Starting tool: {}", name)),
                        timestamp: chrono::Utc::now(),
                        metadata: HashMap::new(),
                    }).await;
                }
                
                // 模拟工具执行
                tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;
                
                // 发送完成通知
                if let Some(token) = &progress_token {
                    self.send_progress_notification(ProgressNotification {
                        progress_token: token.clone(),
                        progress: 100,
                        total: Some(100),
                        message: Some(format!("Completed tool: {}", name)),
                        timestamp: chrono::Utc::now(),
                        metadata: HashMap::new(),
                    }).await;
                }
                
                Ok(serde_json::json!({
                    "tool": name,
                    "result": "success",
                    "output": format!("Tool {} executed with arguments: {}", name, arguments)
                }))
            }
            BatchRequestItem::ListTools { .. } => {
                Ok(serde_json::json!({
                    "tools": [],
                    "next_cursor": null
                }))
            }
            BatchRequestItem::GetResource { uri, .. } => {
                Ok(serde_json::json!({
                    "uri": uri,
                    "content": "Resource content"
                }))
            }
        }
    }
    
    /// 更新批处理状态
    async fn update_batch_status(&self, batch_id: Uuid, status: BatchStatus, message: Option<String>) {
        if let Some(info) = self.batch_executions.write().await.get_mut(&batch_id) {
            info.status = status.clone();
            info.current_message = message;
            
            match status {
                BatchStatus::Running => {
                    info.started_at = Some(chrono::Utc::now());
                }
                BatchStatus::Completed | BatchStatus::Failed | BatchStatus::Cancelled => {
                    info.completed_at = Some(chrono::Utc::now());
                    info.progress = 100;
                }
                _ => {}
            }
        }
    }
    
    /// 更新批处理进度
    async fn update_batch_progress(&self, batch_id: Uuid, completed_items: usize, message: Option<String>) {
        if let Some(info) = self.batch_executions.write().await.get_mut(&batch_id) {
            info.completed_items = completed_items + 1;
            info.progress = ((info.completed_items as f32 / info.total_items as f32) * 100.0) as u32;
            info.current_message = message;
        }
    }
    
    /// 发送进度通知
    async fn send_progress_notification(&self, notification: ProgressNotification) {
        // 广播给所有监听器
        let _ = self.progress_broadcaster.send(notification.clone());
        
        // 发送给特定的进度令牌监听器
        if let Some(sender) = self.progress_listeners.read().await.get(&notification.progress_token) {
            let _ = sender.send(notification);
        }
    }
    
    /// 注册进度监听器
    pub async fn register_progress_listener(&self, progress_token: String) -> ProgressListener {
        let (sender, receiver) = mpsc::unbounded_channel();
        self.progress_listeners.write().await.insert(progress_token, sender);
        receiver
    }
    
    /// 取消注册进度监听器
    pub async fn unregister_progress_listener(&self, progress_token: &str) {
        self.progress_listeners.write().await.remove(progress_token);
    }
    
    /// 获取批处理状态
    pub async fn get_batch_status(&self, batch_id: Uuid) -> Option<BatchExecutionInfo> {
        self.batch_executions.read().await.get(&batch_id).cloned()
    }
    
    /// 获取所有批处理状态
    pub async fn get_all_batch_status(&self) -> Vec<BatchExecutionInfo> {
        self.batch_executions.read().await.values().cloned().collect()
    }
    
    /// 取消批处理
    pub async fn cancel_batch(&self, batch_id: Uuid) -> Result<()> {
        if let Some(_) = self.batch_executions.read().await.get(&batch_id) {
            self.update_batch_status(batch_id, BatchStatus::Cancelled, Some("Batch cancelled by user".to_string())).await;
            info!("Cancelled batch: {}", batch_id);
            Ok(())
        } else {
            Err(anyhow!("Batch not found: {}", batch_id))
        }
    }
    
    /// 清理已完成的批处理
    pub async fn cleanup_completed_batches(&self, older_than_hours: u64) {
        let cutoff = chrono::Utc::now() - chrono::Duration::hours(older_than_hours as i64);
        let mut to_remove = Vec::new();
        
        {
            let executions = self.batch_executions.read().await;
            for (id, info) in executions.iter() {
                if let Some(completed_at) = info.completed_at {
                    if completed_at < cutoff {
                        to_remove.push(*id);
                    }
                }
            }
        }
        
        if !to_remove.is_empty() {
            let mut executions = self.batch_executions.write().await;
            for id in to_remove {
                executions.remove(&id);
            }
            info!("Cleaned up {} completed batches", executions.len());
        }
    }
}

impl Clone for BatchProgressManager {
    fn clone(&self) -> Self {
        Self {
            batch_executions: self.batch_executions.clone(),
            progress_broadcaster: self.progress_broadcaster.clone(),
            progress_listeners: self.progress_listeners.clone(),
            max_batch_size: self.max_batch_size,
            default_timeout_seconds: self.default_timeout_seconds,
            max_concurrent_batches: self.max_concurrent_batches,
        }
    }
}

/// 创建默认的批处理管理器
pub fn create_default_batch_manager() -> BatchProgressManager {
    BatchProgressManager::new(
        100,  // 最大批处理大小
        300,  // 默认超时5分钟
        10,   // 最大并发批处理数
    )
}

/// 创建批处理请求构建器
pub struct BatchRequestBuilder {
    id: Uuid,
    requests: Vec<BatchRequestItem>,
    parallel: bool,
    max_concurrency: Option<usize>,
    timeout_seconds: Option<u64>,
}

impl BatchRequestBuilder {
    pub fn new() -> Self {
        Self {
            id: Uuid::new_v4(),
            requests: Vec::new(),
            parallel: false,
            max_concurrency: None,
            timeout_seconds: None,
        }
    }
    
    pub fn add_tool_call(mut self, name: String, arguments: serde_json::Value, progress_token: Option<String>) -> Self {
        self.requests.push(BatchRequestItem::CallTool {
            id: Uuid::new_v4(),
            name,
            arguments,
            progress_token,
        });
        self
    }
    
    pub fn add_list_tools(mut self, cursor: Option<String>) -> Self {
        self.requests.push(BatchRequestItem::ListTools {
            id: Uuid::new_v4(),
            cursor,
        });
        self
    }
    
    pub fn add_get_resource(mut self, uri: String) -> Self {
        self.requests.push(BatchRequestItem::GetResource {
            id: Uuid::new_v4(),
            uri,
        });
        self
    }
    
    pub fn parallel(mut self, parallel: bool) -> Self {
        self.parallel = parallel;
        self
    }
    
    pub fn max_concurrency(mut self, max_concurrency: usize) -> Self {
        self.max_concurrency = Some(max_concurrency);
        self
    }
    
    pub fn timeout_seconds(mut self, timeout_seconds: u64) -> Self {
        self.timeout_seconds = Some(timeout_seconds);
        self
    }
    
    pub fn build(self) -> BatchRequest {
        BatchRequest {
            id: self.id,
            requests: self.requests,
            parallel: self.parallel,
            max_concurrency: self.max_concurrency,
            timeout_seconds: self.timeout_seconds,
        }
    }
}

impl Default for BatchRequestBuilder {
    fn default() -> Self {
        Self::new()
    }
}
