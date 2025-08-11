use crate::mcp::{types::*, ResourceUsage};

use anyhow::{anyhow, Result};
use std::arch::aarch64::float16x4_t;
use std::collections::HashMap;
use std::sync::Arc;
use std::time::Duration;
use tokio::process::Command;
use tokio::sync::{mpsc, RwLock};
use uuid::Uuid;




/// 工具管理器
pub struct ToolManager {
    tools: Arc<RwLock<HashMap<String, Arc<dyn McpTool>>>>,
    executions: Arc<RwLock<HashMap<Uuid, ToolExecutionResult>>>,
    progress_sender: Option<mpsc::UnboundedSender<ExecutionProgress>>,
}

impl ToolManager {
    pub fn new() -> Self {
        Self {
            tools: Arc::new(RwLock::new(HashMap::new())),
            executions: Arc::new(RwLock::new(HashMap::new())),
            progress_sender: None,
        }
    }

    /// 注册工具
    pub async fn register_tool(&self, tool: Arc<dyn McpTool>) -> Result<()> {
        let mut tools = self.tools.write().await;
        tools.insert(tool.definition().name.clone(), tool);
        Ok(())
    }

    /// 注册多个工具
    pub async fn register_tools(&self, tools: Vec<Arc<dyn McpTool>>) -> Result<()> {
        let mut tool_map = self.tools.write().await;
        for tool in tools {
            tool_map.insert(tool.definition().name.clone(), tool);
        }
        Ok(())
    }

    /// 获取所有工具
    pub async fn get_tools(&self) -> Vec<Arc<dyn McpTool>> {
        let tools = self.tools.read().await;
        tools.values().cloned().collect()
    }

    /// 根据ID获取工具
    pub async fn get_tool(&self, tool_name: &str) -> Option<Arc<dyn McpTool>> {
        let tools = self.tools.read().await;
        tools.get(tool_name).cloned()
    }

    /// 根据分类获取工具
    pub async fn get_tools_by_category(&self, category: ToolCategory) -> Vec<Arc<dyn McpTool>> {
        let tools = self.tools.read().await;
        tools
            .values()
            .filter(|tool| {
                std::mem::discriminant(&tool.definition().category)
                    == std::mem::discriminant(&category)
            })
            .cloned()
            .collect()
    }

    /// 搜索工具
    pub async fn search_tools(&self, query: &str) -> Vec<Arc<dyn McpTool>> {
        let tools = self.tools.read().await;
        let query_lower = query.to_lowercase();

        tools
            .values()
            .filter(|tool| {
                let def = tool.definition();
                def.name.to_lowercase().contains(&query_lower)
                    || def.description.to_lowercase().contains(&query_lower)
                    || def
                        .metadata
                        .tags
                        .iter()
                        .any(|tag| tag.to_lowercase().contains(&query_lower))
            })
            .cloned()
            .collect()
    }

    /// 检查工具是否已安装
    pub async fn is_tool_installed(&self, tool_name: &str) -> Result<bool> {
        if let Some(tool) = self.get_tool(tool_name).await {
            // 简单检查：尝试运行工具的help命令
            let result = Command::new(&tool.definition().name.to_lowercase())
                .arg("--help")
                .output()
                .await;

            Ok(result.is_ok())
        } else {
            Err(anyhow!("Tool does not exist: {}", tool_name))
        }
    }

    /// 安装工具
    pub async fn install_tool(&self, tool_name: &str) -> Result<()> {
        if let Some(tool) = self.get_tool(tool_name).await {
            let def = tool.definition();
            if let Some(install_cmd) = &def.metadata.install_command {
                // 解析安装命令
                let parts: Vec<&str> = install_cmd.split_whitespace().collect();
                if parts.is_empty() {
                    return Err(anyhow!("Install command is empty"));
                }

                let output = Command::new(parts[0]).args(&parts[1..]).output().await?;

                if output.status.success() {
                    tracing::info!("Tool {} installed successfully", def.name);
                    Ok(())
                } else {
                    let error = String::from_utf8_lossy(&output.stderr);
                    Err(anyhow!("Tool installation failed: {}", error))
                }
            } else {
                Err(anyhow!("Tool {} has no install command", def.name))
            }
        } else {
            Err(anyhow!("Tool does not exist: {}", tool_name))
        }
    }

    /// 执行工具
    pub async fn execute_tool(&self, request: ToolExecutionRequest) -> Result<Uuid> {
        let execution_id = Uuid::new_v4();

        let execution_result = ToolExecutionResult {
            execution_id,
            tool_id: request.tool_id.clone(),
            status: ExecutionStatus::Completed,
            output: ExecutionOutput {
                stdout: "Mock output".to_string(),
                stderr: String::new(),
                exit_code: Some(0),
                structured_data: None,
                artifacts: Vec::new(),
            },
            metadata: ExecutionMetadata {
                started_at: chrono::Utc::now(),
                completed_at: Some(chrono::Utc::now()),
                duration: Some(1000),
                resource_usage: ResourceUsage {
                    cpu_time: 10.0,
                    memory_peak: 50 * 1024 * 1024, // 50MB
                    network_requests: 10,
                    disk_io: 0,
                },
            },
        };

        let mut executions = self.executions.write().await;
        executions.insert(execution_id, execution_result);

        Ok(execution_id)
    }

    /// 批量执行工具
    pub async fn execute_batch(&self, request: BatchExecutionRequest) -> Result<Vec<Uuid>> {
        match request.mode {
            BatchMode::Parallel => {
                let mut execution_ids = Vec::new();
                for req in request.requests {
                    let execution_id = self.execute_tool(req).await?;
                    execution_ids.push(execution_id);
                }
                Ok(execution_ids)
            }
            BatchMode::Sequential => {
                let mut execution_ids = Vec::new();
                for req in request.requests {
                    let execution_id = self.execute_tool(req).await?;
                    // 等待执行完成
                    self.wait_for_completion(execution_id).await?;
                    execution_ids.push(execution_id);
                }
                Ok(execution_ids)
            }
            BatchMode::Pipeline => {
                // 管道模式：前一个的输出作为后一个的输入
                let mut execution_ids = Vec::new();
                let mut previous_output: Option<serde_json::Value> = None;

                for mut req in request.requests {
                    // 如果有前一个输出，添加到参数中
                    if let Some(ref output) = previous_output {
                        req.parameters
                            .insert("input_data".to_string(), output.clone());
                    }

                    let execution_id = self.execute_tool(req).await?;
                    self.wait_for_completion(execution_id).await?;

                    // 获取输出用于下一个工具
                    if let Some(result) = self.get_execution_result(execution_id).await {
                        previous_output = result.output.structured_data;
                    }

                    execution_ids.push(execution_id);
                }
                Ok(execution_ids)
            }
        }
    }

    /// 等待执行完成
    async fn wait_for_completion(&self, execution_id: Uuid) -> Result<()> {
        let mut retries = 100; // 最多等待100秒

        while retries > 0 {
            if let Some(result) = self.get_execution_result(execution_id).await {
                match result.status {
                    ExecutionStatus::Completed
                    | ExecutionStatus::Failed
                    | ExecutionStatus::Cancelled
                    | ExecutionStatus::Timeout => {
                        return Ok(());
                    }
                    _ => {
                        tokio::time::sleep(Duration::from_secs(1)).await;
                        retries -= 1;
                    }
                }
            } else {
                return Err(anyhow!("Execution record does not exist"));
            }
        }

        Err(anyhow!("Timeout waiting for execution to complete"))
    }

    /// 获取执行结果
    pub async fn get_execution_result(&self, execution_id: Uuid) -> Option<ToolExecutionResult> {
        let executions = self.executions.read().await;
        executions.get(&execution_id).cloned()
    }

    /// 获取所有执行记录
    pub async fn get_execution_history(&self) -> Vec<ToolExecutionResult> {
        let executions = self.executions.read().await;
        executions.values().cloned().collect()
    }

    /// 取消执行
    pub async fn cancel_execution(&self, execution_id: Uuid) -> Result<()> {
        let mut executions = self.executions.write().await;
        if let Some(mut result) = executions.get(&execution_id).cloned() {
            result.status = ExecutionStatus::Cancelled;
            result.metadata.completed_at = Some(chrono::Utc::now());
            executions.insert(execution_id, result);
            Ok(())
        } else {
            Err(anyhow!("Execution record does not exist"))
        }
    }

    /// 清理执行历史
    pub async fn cleanup_executions(&self, older_than_hours: u64) -> Result<usize> {
        let cutoff_time = chrono::Utc::now() - chrono::Duration::hours(older_than_hours as i64);
        let mut executions = self.executions.write().await;
        let initial_count = executions.len();

        executions.retain(|_, result| result.metadata.started_at > cutoff_time);

        Ok(initial_count - executions.len())
    }

    /// 获取工具统计信息
    pub async fn get_tool_statistics(&self) -> HashMap<String, ToolStatistics> {
        let executions = self.executions.read().await;
        let mut stats: HashMap<String, ToolStatistics> = HashMap::new();

        for result in executions.values() {
            let stat = stats
                .entry(result.tool_id.clone())
                .or_insert(ToolStatistics {
                    tool_id: result.tool_id.clone(),
                    total_executions: 0,
                    successful_executions: 0,
                    failed_executions: 0,
                    average_duration_ms: 0,
                    last_execution: None,
                });

            stat.total_executions += 1;
            match result.status {
                ExecutionStatus::Completed => stat.successful_executions += 1,
                ExecutionStatus::Failed => stat.failed_executions += 1,
                _ => {}
            }

            if let Some(duration) = result.metadata.duration {
                stat.average_duration_ms = (stat.average_duration_ms * (stat.total_executions - 1)
                    + duration)
                    / stat.total_executions;
            }

            if stat.last_execution.is_none()
                || stat.last_execution.as_ref().unwrap() < &result.metadata.started_at
            {
                stat.last_execution = Some(result.metadata.started_at);
            }
        }

        stats
    }

    /// 设置进度监听器
    pub fn set_progress_listener(&mut self, sender: mpsc::UnboundedSender<ExecutionProgress>) {
        self.progress_sender = Some(sender);
    }
}

/// 工具统计信息
#[derive(Debug, Clone)]
pub struct ToolStatistics {
    pub tool_id: String,
    pub total_executions: u64,
    pub successful_executions: u64,
    pub failed_executions: u64,
    pub average_duration_ms: u64,
    pub last_execution: Option<chrono::DateTime<chrono::Utc>>,
}

impl Default for ToolManager {
    fn default() -> Self {
        Self::new()
    }
}
