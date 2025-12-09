//! Bash 相关工具
//!
//! 参考 Claude Code 的 Bash、BashOutput、KillShell 工具实现
//!
//! ## 工具列表
//! - `bash`: 执行 shell 命令
//! - `bash_output`: 获取后台 shell 输出
//! - `kill_shell`: 终止后台 shell

use super::shell_manager::{get_global_shell_manager, ShellManager, ShellStatus};
use crate::tools::*;
use anyhow::{anyhow, Result};
use async_trait::async_trait;
use serde_json::json;
use std::collections::HashMap;
use std::sync::Arc;
use uuid::Uuid;

// ============================================================================
// Bash 工具 - 执行 shell 命令
// ============================================================================

/// Bash 工具
///
/// 在持久 shell 会话中执行命令，支持：
/// - 前台执行（等待完成）
/// - 后台执行（立即返回）
/// - 超时控制
#[derive(Debug)]
pub struct BashTool {
    metadata: ToolMetadata,
    parameters: ToolParameters,
    shell_manager: Arc<ShellManager>,
}

impl BashTool {
    pub fn new() -> Self {
        Self::with_manager(Arc::new(ShellManager::new()))
    }

    pub fn with_manager(shell_manager: Arc<ShellManager>) -> Self {
        let metadata = ToolMetadata {
            author: "Built-in".to_string(),
            version: "1.0.0".to_string(),
            license: "MIT".to_string(),
            homepage: None,
            repository: None,
            tags: vec![
                "bash".to_string(),
                "shell".to_string(),
                "command".to_string(),
                "terminal".to_string(),
            ],
            install_command: None,
            requirements: vec![],
        };

        let parameters = ToolParameters {
            parameters: vec![
                ParameterDefinition {
                    name: "command".to_string(),
                    param_type: ParameterType::String,
                    description: "The command to execute".to_string(),
                    required: true,
                    default_value: None,
                },
                ParameterDefinition {
                    name: "timeout".to_string(),
                    param_type: ParameterType::Number,
                    description: "Optional timeout in milliseconds (max 600000, default 120000)"
                        .to_string(),
                    required: false,
                    default_value: Some(json!(120000)),
                },
                ParameterDefinition {
                    name: "description".to_string(),
                    param_type: ParameterType::String,
                    description: "Clear, concise description of what this command does (5-10 words)"
                        .to_string(),
                    required: false,
                    default_value: None,
                },
                ParameterDefinition {
                    name: "run_in_background".to_string(),
                    param_type: ParameterType::Boolean,
                    description:
                        "Set to true to run command in background. Use bash_output to read output later."
                            .to_string(),
                    required: false,
                    default_value: Some(json!(false)),
                },
            ],
            schema: json!({
                "type": "object",
                "properties": {
                    "command": {"type": "string", "description": "The command to execute"},
                    "timeout": {"type": "number", "description": "Timeout in milliseconds (max 600000)"},
                    "description": {"type": "string", "description": "Command description (5-10 words)"},
                    "run_in_background": {"type": "boolean", "description": "Run in background"}
                },
                "required": ["command"]
            }),
            required: vec!["command".to_string()],
            optional: vec![
                "timeout".to_string(),
                "description".to_string(),
                "run_in_background".to_string(),
            ],
        };

        Self {
            metadata,
            parameters,
            shell_manager,
        }
    }
}

#[async_trait]
impl UnifiedTool for BashTool {
    fn name(&self) -> &str {
        "bash"
    }

    fn description(&self) -> &str {
        "Executes a given bash command in a persistent shell session with optional timeout. \
         Use run_in_background=true for long-running commands."
    }

    fn category(&self) -> ToolCategory {
        ToolCategory::Utility
    }

    fn parameters(&self) -> &ToolParameters {
        &self.parameters
    }

    fn metadata(&self) -> &ToolMetadata {
        &self.metadata
    }

    async fn is_available(&self) -> bool {
        true
    }

    async fn execute(&self, params: ToolExecutionParams) -> Result<ToolExecutionResult> {
        let execution_id = params.execution_id.unwrap_or_else(Uuid::new_v4);
        let start_time = std::time::Instant::now();

        // 获取参数
        let command = params
            .inputs
            .get("command")
            .and_then(|v| v.as_str())
            .ok_or_else(|| anyhow!("command parameter is required"))?;

        if command.is_empty() {
            return Err(anyhow!("command cannot be empty"));
        }

        let timeout_ms = params
            .inputs
            .get("timeout")
            .and_then(|v| v.as_u64())
            .unwrap_or(120_000)
            .min(600_000); // 最大 10 分钟

        let description = params
            .inputs
            .get("description")
            .and_then(|v| v.as_str());

        let run_in_background = params
            .inputs
            .get("run_in_background")
            .and_then(|v| v.as_bool())
            .unwrap_or(false);

        tracing::info!(
            "Bash: {} (background: {}, timeout: {}ms)",
            truncate_for_log(command, 50),
            run_in_background,
            timeout_ms
        );

        if run_in_background {
            // 后台执行
            match self
                .shell_manager
                .execute_background(command, description)
                .await
            {
                Ok(shell_id) => {
                    let execution_time_ms = start_time.elapsed().as_millis() as u64;
                    Ok(ToolExecutionResult {
                        execution_id,
                        tool_id: self.name().to_string(),
                        tool_name: self.name().to_string(),
                        success: true,
                        output: json!({
                            "shell_id": shell_id,
                            "status": "running",
                            "message": "Command started in background. Use bash_output to get output."
                        }),
                        error: None,
                        execution_time_ms,
                        metadata: HashMap::new(),
                        status: ExecutionStatus::Completed,
                        started_at: chrono::Utc::now(),
                        completed_at: Some(chrono::Utc::now()),
                    })
                }
                Err(e) => {
                    let execution_time_ms = start_time.elapsed().as_millis() as u64;
                    Ok(ToolExecutionResult {
                        execution_id,
                        tool_id: self.name().to_string(),
                        tool_name: self.name().to_string(),
                        success: false,
                        output: json!({}),
                        error: Some(e.to_string()),
                        execution_time_ms,
                        metadata: HashMap::new(),
                        status: ExecutionStatus::Failed,
                        started_at: chrono::Utc::now(),
                        completed_at: Some(chrono::Utc::now()),
                    })
                }
            }
        } else {
            // 前台执行
            match self
                .shell_manager
                .execute(command, Some(timeout_ms), description)
                .await
            {
                Ok(result) => {
                    let execution_time_ms = start_time.elapsed().as_millis() as u64;
                    let success = result.status == ShellStatus::Completed;

                    Ok(ToolExecutionResult {
                        execution_id,
                        tool_id: self.name().to_string(),
                        tool_name: self.name().to_string(),
                        success,
                        output: json!({
                            "stdout": result.stdout,
                            "stderr": result.stderr,
                            "exit_code": result.exit_code,
                            "status": result.status.to_string(),
                            "duration_ms": result.duration_ms
                        }),
                        error: if success { None } else { Some(result.stderr.clone()) },
                        execution_time_ms,
                        metadata: HashMap::new(),
                        status: if success {
                            ExecutionStatus::Completed
                        } else {
                            ExecutionStatus::Failed
                        },
                        started_at: chrono::Utc::now(),
                        completed_at: Some(chrono::Utc::now()),
                    })
                }
                Err(e) => {
                    let execution_time_ms = start_time.elapsed().as_millis() as u64;
                    Ok(ToolExecutionResult {
                        execution_id,
                        tool_id: self.name().to_string(),
                        tool_name: self.name().to_string(),
                        success: false,
                        output: json!({}),
                        error: Some(e.to_string()),
                        execution_time_ms,
                        metadata: HashMap::new(),
                        status: ExecutionStatus::Failed,
                        started_at: chrono::Utc::now(),
                        completed_at: Some(chrono::Utc::now()),
                    })
                }
            }
        }
    }
}

// ============================================================================
// BashOutput 工具 - 获取后台 shell 输出
// ============================================================================

/// BashOutput 工具
///
/// 获取后台运行的 shell 输出
#[derive(Debug)]
pub struct BashOutputTool {
    metadata: ToolMetadata,
    parameters: ToolParameters,
    shell_manager: Arc<ShellManager>,
}

impl BashOutputTool {
    pub fn new() -> Self {
        Self::with_manager(Arc::new(ShellManager::new()))
    }

    pub fn with_manager(shell_manager: Arc<ShellManager>) -> Self {
        let metadata = ToolMetadata {
            author: "Built-in".to_string(),
            version: "1.0.0".to_string(),
            license: "MIT".to_string(),
            homepage: None,
            repository: None,
            tags: vec![
                "bash".to_string(),
                "output".to_string(),
                "background".to_string(),
            ],
            install_command: None,
            requirements: vec![],
        };

        let parameters = ToolParameters {
            parameters: vec![
                ParameterDefinition {
                    name: "bash_id".to_string(),
                    param_type: ParameterType::String,
                    description: "The ID of the background shell to retrieve output from"
                        .to_string(),
                    required: true,
                    default_value: None,
                },
                ParameterDefinition {
                    name: "filter".to_string(),
                    param_type: ParameterType::String,
                    description:
                        "Optional regex to filter output lines. Non-matching lines are discarded."
                            .to_string(),
                    required: false,
                    default_value: None,
                },
            ],
            schema: json!({
                "type": "object",
                "properties": {
                    "bash_id": {"type": "string", "description": "Background shell ID"},
                    "filter": {"type": "string", "description": "Regex filter for output"}
                },
                "required": ["bash_id"]
            }),
            required: vec!["bash_id".to_string()],
            optional: vec!["filter".to_string()],
        };

        Self {
            metadata,
            parameters,
            shell_manager,
        }
    }
}

#[async_trait]
impl UnifiedTool for BashOutputTool {
    fn name(&self) -> &str {
        "bash_output"
    }

    fn description(&self) -> &str {
        "Retrieves output from a running or completed background bash shell. \
         Returns only new output since last check. Use filter for regex matching."
    }

    fn category(&self) -> ToolCategory {
        ToolCategory::Utility
    }

    fn parameters(&self) -> &ToolParameters {
        &self.parameters
    }

    fn metadata(&self) -> &ToolMetadata {
        &self.metadata
    }

    async fn is_available(&self) -> bool {
        true
    }

    async fn execute(&self, params: ToolExecutionParams) -> Result<ToolExecutionResult> {
        let execution_id = params.execution_id.unwrap_or_else(Uuid::new_v4);
        let start_time = std::time::Instant::now();

        let bash_id = params
            .inputs
            .get("bash_id")
            .and_then(|v| v.as_str())
            .ok_or_else(|| anyhow!("bash_id parameter is required"))?;

        let filter = params.inputs.get("filter").and_then(|v| v.as_str());

        match self.shell_manager.get_output(bash_id, filter).await {
            Ok(result) => {
                let execution_time_ms = start_time.elapsed().as_millis() as u64;
                Ok(ToolExecutionResult {
                    execution_id,
                    tool_id: self.name().to_string(),
                    tool_name: self.name().to_string(),
                    success: true,
                    output: json!({
                        "shell_id": result.shell_id,
                        "status": result.status.to_string(),
                        "stdout": result.stdout,
                        "stderr": result.stderr,
                        "exit_code": result.exit_code,
                        "duration_ms": result.duration_ms
                    }),
                    error: None,
                    execution_time_ms,
                    metadata: HashMap::new(),
                    status: ExecutionStatus::Completed,
                    started_at: chrono::Utc::now(),
                    completed_at: Some(chrono::Utc::now()),
                })
            }
            Err(e) => {
                let execution_time_ms = start_time.elapsed().as_millis() as u64;
                Ok(ToolExecutionResult {
                    execution_id,
                    tool_id: self.name().to_string(),
                    tool_name: self.name().to_string(),
                    success: false,
                    output: json!({}),
                    error: Some(e.to_string()),
                    execution_time_ms,
                    metadata: HashMap::new(),
                    status: ExecutionStatus::Failed,
                    started_at: chrono::Utc::now(),
                    completed_at: Some(chrono::Utc::now()),
                })
            }
        }
    }
}

// ============================================================================
// KillShell 工具 - 终止后台 shell
// ============================================================================

/// KillShell 工具
///
/// 终止后台运行的 shell 进程
#[derive(Debug)]
pub struct KillShellTool {
    metadata: ToolMetadata,
    parameters: ToolParameters,
    shell_manager: Arc<ShellManager>,
}

impl KillShellTool {
    pub fn new() -> Self {
        Self::with_manager(Arc::new(ShellManager::new()))
    }

    pub fn with_manager(shell_manager: Arc<ShellManager>) -> Self {
        let metadata = ToolMetadata {
            author: "Built-in".to_string(),
            version: "1.0.0".to_string(),
            license: "MIT".to_string(),
            homepage: None,
            repository: None,
            tags: vec!["bash".to_string(), "kill".to_string(), "process".to_string()],
            install_command: None,
            requirements: vec![],
        };

        let parameters = ToolParameters {
            parameters: vec![ParameterDefinition {
                name: "shell_id".to_string(),
                param_type: ParameterType::String,
                description: "The ID of the background shell to kill".to_string(),
                required: true,
                default_value: None,
            }],
            schema: json!({
                "type": "object",
                "properties": {
                    "shell_id": {"type": "string", "description": "Background shell ID to kill"}
                },
                "required": ["shell_id"]
            }),
            required: vec!["shell_id".to_string()],
            optional: vec![],
        };

        Self {
            metadata,
            parameters,
            shell_manager,
        }
    }
}

#[async_trait]
impl UnifiedTool for KillShellTool {
    fn name(&self) -> &str {
        "kill_shell"
    }

    fn description(&self) -> &str {
        "Kills a running background bash shell by its ID. Returns success or failure status."
    }

    fn category(&self) -> ToolCategory {
        ToolCategory::Utility
    }

    fn parameters(&self) -> &ToolParameters {
        &self.parameters
    }

    fn metadata(&self) -> &ToolMetadata {
        &self.metadata
    }

    async fn is_available(&self) -> bool {
        true
    }

    async fn execute(&self, params: ToolExecutionParams) -> Result<ToolExecutionResult> {
        let execution_id = params.execution_id.unwrap_or_else(Uuid::new_v4);
        let start_time = std::time::Instant::now();

        let shell_id = params
            .inputs
            .get("shell_id")
            .and_then(|v| v.as_str())
            .ok_or_else(|| anyhow!("shell_id parameter is required"))?;

        match self.shell_manager.kill(shell_id).await {
            Ok(()) => {
                let execution_time_ms = start_time.elapsed().as_millis() as u64;
                Ok(ToolExecutionResult {
                    execution_id,
                    tool_id: self.name().to_string(),
                    tool_name: self.name().to_string(),
                    success: true,
                    output: json!({
                        "shell_id": shell_id,
                        "status": "killed",
                        "message": "Shell terminated successfully"
                    }),
                    error: None,
                    execution_time_ms,
                    metadata: HashMap::new(),
                    status: ExecutionStatus::Completed,
                    started_at: chrono::Utc::now(),
                    completed_at: Some(chrono::Utc::now()),
                })
            }
            Err(e) => {
                let execution_time_ms = start_time.elapsed().as_millis() as u64;
                Ok(ToolExecutionResult {
                    execution_id,
                    tool_id: self.name().to_string(),
                    tool_name: self.name().to_string(),
                    success: false,
                    output: json!({}),
                    error: Some(e.to_string()),
                    execution_time_ms,
                    metadata: HashMap::new(),
                    status: ExecutionStatus::Failed,
                    started_at: chrono::Utc::now(),
                    completed_at: Some(chrono::Utc::now()),
                })
            }
        }
    }
}

// ============================================================================
// 辅助函数
// ============================================================================

fn truncate_for_log(s: &str, max_len: usize) -> String {
    if s.len() <= max_len {
        s.to_string()
    } else {
        format!("{}...", &s[..max_len - 3])
    }
}

// ============================================================================
// 工厂函数 - 创建共享 ShellManager 的工具集
// ============================================================================

/// 创建共享 ShellManager 的 Bash 工具集
pub fn create_bash_tools() -> (BashTool, BashOutputTool, KillShellTool) {
    let shell_manager = Arc::new(ShellManager::new());
    (
        BashTool::with_manager(shell_manager.clone()),
        BashOutputTool::with_manager(shell_manager.clone()),
        KillShellTool::with_manager(shell_manager),
    )
}

/// 使用指定 ShellManager 创建工具集
pub fn create_bash_tools_with_manager(
    shell_manager: Arc<ShellManager>,
) -> (BashTool, BashOutputTool, KillShellTool) {
    (
        BashTool::with_manager(shell_manager.clone()),
        BashOutputTool::with_manager(shell_manager.clone()),
        KillShellTool::with_manager(shell_manager),
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_bash_tool_echo() {
        let tool = BashTool::new();

        let params = ToolExecutionParams {
            inputs: json!({
                "command": "echo hello world",
                "description": "Print hello world"
            })
            .as_object()
            .unwrap()
            .iter()
            .map(|(k, v)| (k.clone(), v.clone()))
            .collect(),
            execution_id: None,
            timeout: None,
            context: None,
        };

        let result = tool.execute(params).await.unwrap();
        assert!(result.success);
        assert!(result.output["stdout"]
            .as_str()
            .unwrap()
            .contains("hello world"));
    }

    #[tokio::test]
    async fn test_bash_tool_background() {
        let shell_manager = Arc::new(ShellManager::new());
        let bash_tool = BashTool::with_manager(shell_manager.clone());
        let output_tool = BashOutputTool::with_manager(shell_manager.clone());

        // 启动后台命令
        let params = ToolExecutionParams {
            inputs: json!({
                "command": "sleep 1 && echo background_done",
                "run_in_background": true
            })
            .as_object()
            .unwrap()
            .iter()
            .map(|(k, v)| (k.clone(), v.clone()))
            .collect(),
            execution_id: None,
            timeout: None,
            context: None,
        };

        let result = bash_tool.execute(params).await.unwrap();
        assert!(result.success);

        let shell_id = result.output["shell_id"].as_str().unwrap();

        // 等待完成
        tokio::time::sleep(tokio::time::Duration::from_secs(2)).await;

        // 获取输出
        let params = ToolExecutionParams {
            inputs: json!({
                "bash_id": shell_id
            })
            .as_object()
            .unwrap()
            .iter()
            .map(|(k, v)| (k.clone(), v.clone()))
            .collect(),
            execution_id: None,
            timeout: None,
            context: None,
        };

        let result = output_tool.execute(params).await.unwrap();
        assert!(result.success);
        assert_eq!(result.output["status"].as_str().unwrap(), "completed");
    }

    #[tokio::test]
    async fn test_kill_shell_tool() {
        let shell_manager = Arc::new(ShellManager::new());
        let bash_tool = BashTool::with_manager(shell_manager.clone());
        let kill_tool = KillShellTool::with_manager(shell_manager.clone());

        // 启动长时间运行的后台命令
        let params = ToolExecutionParams {
            inputs: json!({
                "command": "sleep 100",
                "run_in_background": true
            })
            .as_object()
            .unwrap()
            .iter()
            .map(|(k, v)| (k.clone(), v.clone()))
            .collect(),
            execution_id: None,
            timeout: None,
            context: None,
        };

        let result = bash_tool.execute(params).await.unwrap();
        let shell_id = result.output["shell_id"].as_str().unwrap();

        // 终止
        let params = ToolExecutionParams {
            inputs: json!({
                "shell_id": shell_id
            })
            .as_object()
            .unwrap()
            .iter()
            .map(|(k, v)| (k.clone(), v.clone()))
            .collect(),
            execution_id: None,
            timeout: None,
            context: None,
        };

        let result = kill_tool.execute(params).await.unwrap();
        assert!(result.success);
        assert_eq!(result.output["status"].as_str().unwrap(), "killed");
    }
}

