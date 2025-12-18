//! Shell command execution tool using rig-core Tool trait

use rig::tool::Tool;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use std::process::Stdio;
use std::sync::Arc;
use std::time::Instant;
use tokio::process::Command;
use tokio::sync::RwLock;
use tokio::time::{timeout, Duration};
use once_cell::sync::Lazy;

/// Shell command arguments
#[derive(Debug, Clone, Deserialize, JsonSchema)]
pub struct ShellArgs {
    /// Shell command to execute
    pub command: String,
    /// Working directory (optional)
    #[serde(default)]
    pub cwd: Option<String>,
    /// Command timeout in seconds
    #[serde(default = "default_timeout")]
    pub timeout_secs: u64,
}

fn default_timeout() -> u64 { 60 }

/// Shell command result
#[derive(Debug, Clone, Serialize)]
pub struct ShellOutput {
    pub command: String,
    pub stdout: String,
    pub stderr: String,
    pub exit_code: Option<i32>,
    pub success: bool,
    pub execution_time_ms: u64,
}

/// Shell command errors
#[derive(Debug, thiserror::Error)]
pub enum ShellError {
    #[error("Command execution failed: {0}")]
    ExecutionFailed(String),
    #[error("Command timeout after {0} seconds")]
    Timeout(u64),
    #[error("Invalid command: {0}")]
    InvalidCommand(String),
    #[error("Permission denied: {0}")]
    PermissionDenied(String),
}

/// Shell permission action
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, JsonSchema)]
pub enum ShellPermissionAction {
    Allow,
    Deny,
    Ask,
}

/// Shell permission rule
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct ShellRule {
    pub pattern: String,
    pub action: ShellPermissionAction,
}

/// Shell configuration
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct ShellConfig {
    pub rules: Vec<ShellRule>,
    pub default_action: ShellPermissionAction,
}

impl Default for ShellConfig {
    fn default() -> Self {
        Self {
            rules: vec![
                ShellRule {
                    pattern: "rm -rf /".to_string(),
                    action: ShellPermissionAction::Deny,
                },
                ShellRule {
                    pattern: "mkfs".to_string(),
                    action: ShellPermissionAction::Deny,
                },
                ShellRule {
                    pattern: "dd if=/dev/zero".to_string(),
                    action: ShellPermissionAction::Deny,
                },
            ],
            default_action: ShellPermissionAction::Ask, // Default to Ask for safety
        }
    }
}

/// Trait for handling permission requests (implemented by app layer)
#[async_trait::async_trait]
pub trait ShellPermissionHandler: Send + Sync {
    async fn check_permission(&self, command: &str) -> bool;
}

/// Global shell configuration
static SHELL_CONFIG: Lazy<RwLock<ShellConfig>> = Lazy::new(|| RwLock::new(ShellConfig::default()));

/// Global permission handler
static PERMISSION_HANDLER: Lazy<RwLock<Option<Arc<dyn ShellPermissionHandler>>>> = Lazy::new(|| RwLock::new(None));

/// Set the global permission handler
pub async fn set_permission_handler(handler: Arc<dyn ShellPermissionHandler>) {
    let mut h = PERMISSION_HANDLER.write().await;
    *h = Some(handler);
}

/// Get current shell config
pub async fn get_shell_config() -> ShellConfig {
    SHELL_CONFIG.read().await.clone()
}

/// Update shell config
pub async fn set_shell_config(config: ShellConfig) {
    let mut c = SHELL_CONFIG.write().await;
    *c = config;
}

/// Shell command tool
#[derive(Debug, Clone)]
pub struct ShellTool;

impl Default for ShellTool {
    fn default() -> Self {
        Self::new()
    }
}

impl ShellTool {
    pub fn new() -> Self {
        Self
    }

    /// Validate command permissions
    async fn check_permission(&self, cmd: &str) -> Result<(), ShellError> {
        let config = SHELL_CONFIG.read().await;
        
        // Check rules
        for rule in &config.rules {
            if cmd.contains(&rule.pattern) {
                match rule.action {
                    ShellPermissionAction::Allow => return Ok(()),
                    ShellPermissionAction::Deny => return Err(ShellError::PermissionDenied(format!("Command denied by rule: {}", rule.pattern))),
                    ShellPermissionAction::Ask => return self.ask_permission(cmd).await,
                }
            }
        }

        // Check default action
        match config.default_action {
            ShellPermissionAction::Allow => Ok(()),
            ShellPermissionAction::Deny => Err(ShellError::PermissionDenied("Command denied by default policy".to_string())),
            ShellPermissionAction::Ask => self.ask_permission(cmd).await,
        }
    }

    async fn ask_permission(&self, cmd: &str) -> Result<(), ShellError> {
        let handler_guard = PERMISSION_HANDLER.read().await;
        if let Some(handler) = &*handler_guard {
            if handler.check_permission(cmd).await {
                Ok(())
            } else {
                Err(ShellError::PermissionDenied("User rejected execution".to_string()))
            }
        } else {
            // If no handler is registered, deny by default for safety
            Err(ShellError::PermissionDenied("No permission handler registered to ask user".to_string()))
        }
    }
}

impl Tool for ShellTool {
    const NAME: &'static str = "shell";
    type Args = ShellArgs;
    type Output = ShellOutput;
    type Error = ShellError;

    async fn definition(&self, _prompt: String) -> rig::completion::ToolDefinition {
        rig::completion::ToolDefinition {
            name: Self::NAME.to_string(),
            description: "Execute shell commands. Use with caution - commands are subject to security policies.".to_string(),
            parameters: serde_json::to_value(schemars::schema_for!(ShellArgs))
                .unwrap_or_default(),
        }
    }

    async fn call(&self, args: Self::Args) -> Result<Self::Output, Self::Error> {
        let start_time = Instant::now();

        // Validate command permission
        self.check_permission(&args.command).await?;

        // Determine shell
        #[cfg(target_os = "windows")]
        let (shell, shell_arg) = ("cmd", "/C");
        #[cfg(not(target_os = "windows"))]
        let (shell, shell_arg) = ("bash", "-c");

        // Build command
        let mut cmd = Command::new(shell);
        cmd.arg(shell_arg);
        cmd.arg(&args.command);
        cmd.stdout(Stdio::piped());
        cmd.stderr(Stdio::piped());

        if let Some(cwd) = &args.cwd {
            cmd.current_dir(cwd);
        }

        // Execute with timeout
        let timeout_duration = Duration::from_secs(args.timeout_secs);
        let result = timeout(timeout_duration, cmd.output()).await;

        match result {
            Ok(Ok(output)) => {
                let stdout = String::from_utf8_lossy(&output.stdout).to_string();
                let stderr = String::from_utf8_lossy(&output.stderr).to_string();
                let exit_code = output.status.code();
                let success = output.status.success();
                let execution_time_ms = start_time.elapsed().as_millis() as u64;

                Ok(ShellOutput {
                    command: args.command,
                    stdout,
                    stderr,
                    exit_code,
                    success,
                    execution_time_ms,
                })
            }
            Ok(Err(e)) => Err(ShellError::ExecutionFailed(e.to_string())),
            Err(_) => Err(ShellError::Timeout(args.timeout_secs)),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_default_rules() {
        let tool = ShellTool::new();
        
        // Denied by default rule
        assert!(matches!(
            tool.check_permission("rm -rf /").await,
            Err(ShellError::PermissionDenied(_))
        ));
    }
}
