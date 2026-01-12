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
use crate::docker_sandbox::{DockerSandbox, DockerSandboxConfig};

/// Shell execution mode
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, JsonSchema)]
pub enum ShellExecutionMode {
    /// Execute on host machine (less secure)
    Host,
    /// Execute in Docker container (more secure)
    Docker,
}

impl Default for ShellExecutionMode {
    fn default() -> Self {
        Self::Docker
    }
}

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
    /// Execution mode (host or docker)
    #[serde(default)]
    pub execution_mode: Option<ShellExecutionMode>,
}

fn default_timeout() -> u64 { 180 }

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
    #[error("Docker error: {0}")]
    DockerError(String),
}

/// Shell default policy
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, JsonSchema)]
#[derive(Default)]
pub enum ShellDefaultPolicy {
    /// Always proceed without asking (except denied commands)
    AlwaysProceed,
    /// Always ask for confirmation (except allowed commands)
    #[default]
    RequestReview,
}

/// Shell configuration (Cursor-style)
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct ShellConfig {
    /// Default policy for commands not in allow/deny lists
    #[serde(default)]
    pub default_policy: ShellDefaultPolicy,
    /// Commands that are auto-allowed (prefix match)
    #[serde(default)]
    pub allowed_commands: Vec<String>,
    /// Commands that are always denied (prefix match, takes precedence)
    #[serde(default)]
    pub denied_commands: Vec<String>,
    /// Default execution mode
    #[serde(default)]
    pub default_execution_mode: ShellExecutionMode,
    /// Docker sandbox configuration
    #[serde(default)]
    pub docker_config: Option<DockerSandboxConfig>,
}

impl Default for ShellConfig {
    fn default() -> Self {
        Self {
            default_policy: ShellDefaultPolicy::RequestReview,
            allowed_commands: vec![],
            denied_commands: vec![
                "rm".to_string(),
                "rm -rf".to_string(),
                "mkfs".to_string(),
                "dd".to_string(),
            ],
            default_execution_mode: ShellExecutionMode::Docker,
            docker_config: Some(DockerSandboxConfig::default()),
        }
    }
}

impl ShellConfig {
    /// Check if a command should be auto-allowed
    pub fn is_allowed(&self, command: &str) -> bool {
        // Denied list takes precedence
        if self.is_denied(command) {
            return false;
        }
        
        // Check allowed list
        for allowed in &self.allowed_commands {
            if command_matches_pattern(command, allowed) {
                return true;
            }
        }
        
        // If AlwaysProceed, allow by default
        self.default_policy == ShellDefaultPolicy::AlwaysProceed
    }
    
    /// Check if a command should be denied
    pub fn is_denied(&self, command: &str) -> bool {
        for denied in &self.denied_commands {
            if command_matches_pattern(command, denied) {
                return true;
            }
        }
        false
    }
    
    /// Check if a command needs user confirmation
    pub fn needs_confirmation(&self, command: &str) -> bool {
        // Denied commands always need confirmation (or rejection)
        if self.is_denied(command) {
            return true;
        }
        
        // Allowed commands don't need confirmation
        for allowed in &self.allowed_commands {
            if command_matches_pattern(command, allowed) {
                return false;
            }
        }
        
        // Default policy determines
        self.default_policy == ShellDefaultPolicy::RequestReview
    }
}

/// Check if command matches pattern (prefix match by tokens)
fn command_matches_pattern(command: &str, pattern: &str) -> bool {
    let cmd_tokens: Vec<&str> = command.split_whitespace().collect();
    let pattern_tokens: Vec<&str> = pattern.split_whitespace().collect();
    
    if pattern_tokens.is_empty() {
        return false;
    }
    
    // Check if pattern tokens form a prefix of command tokens
    if cmd_tokens.len() < pattern_tokens.len() {
        return false;
    }
    
    for (i, pt) in pattern_tokens.iter().enumerate() {
        if cmd_tokens[i] != *pt {
            return false;
        }
    }
    
    true
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

    pub const NAME: &'static str = "shell";
    pub const DESCRIPTION: &'static str = "Execute shell commands in isolated Docker sandbox or on host. Commands are subject to security policies.";

    /// Execute command in Docker sandbox
    async fn execute_in_docker(
        &self,
        cmd: &str,
        timeout_secs: u64,
    ) -> Result<(String, String, i32), ShellError> {
        let config = SHELL_CONFIG.read().await;
        let docker_config = config
            .docker_config
            .clone()
            .unwrap_or_default();
        drop(config);

        let sandbox = DockerSandbox::new(docker_config);
        sandbox
            .execute(cmd, timeout_secs)
            .await
            .map_err(|e| ShellError::DockerError(e.to_string()))
    }

    /// Execute command on host machine
    async fn execute_on_host(
        &self,
        cmd: &str,
        cwd: Option<&str>,
        timeout_secs: u64,
    ) -> Result<(String, String, i32), ShellError> {
        // Determine shell
        #[cfg(target_os = "windows")]
        let (shell, shell_arg) = ("cmd", "/C");
        #[cfg(not(target_os = "windows"))]
        let (shell, shell_arg) = ("bash", "-c");

        // Build command
        let mut command = Command::new(shell);
        command.arg(shell_arg);
        command.arg(cmd);
        command.stdout(Stdio::piped());
        command.stderr(Stdio::piped());

        if let Some(cwd) = cwd {
            command.current_dir(cwd);
        }

        // Execute with timeout
        let timeout_duration = Duration::from_secs(timeout_secs);
        let result = timeout(timeout_duration, command.output()).await;

        match result {
            Ok(Ok(output)) => {
                let stdout = String::from_utf8_lossy(&output.stdout).to_string();
                let stderr = String::from_utf8_lossy(&output.stderr).to_string();
                let exit_code = output.status.code().unwrap_or(-1);
                Ok((stdout, stderr, exit_code))
            }
            Ok(Err(e)) => Err(ShellError::ExecutionFailed(e.to_string())),
            Err(_) => Err(ShellError::Timeout(timeout_secs)),
        }
    }

    /// Validate command permissions
    async fn check_permission(&self, cmd: &str) -> Result<(), ShellError> {
        let config = SHELL_CONFIG.read().await;
        
        // Check if command is in deny list (always deny these)
        if config.is_denied(cmd) {
            return Err(ShellError::PermissionDenied(format!("Command denied by policy: {}", cmd)));
        }
        
        // Check if command is auto-allowed
        if config.is_allowed(cmd) {
            return Ok(());
        }
        
        // Check if needs confirmation based on policy
        if config.needs_confirmation(cmd) {
            return self.ask_permission(cmd).await;
        }
        
        Ok(())
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
    const NAME: &'static str = Self::NAME;
    type Args = ShellArgs;
    type Output = ShellOutput;
    type Error = ShellError;

    async fn definition(&self, _prompt: String) -> rig::completion::ToolDefinition {
        rig::completion::ToolDefinition {
            name: Self::NAME.to_string(),
            description: Self::DESCRIPTION.to_string(),
            parameters: serde_json::to_value(schemars::schema_for!(ShellArgs))
                .unwrap_or_default(),
        }
    }

    async fn call(&self, args: Self::Args) -> Result<Self::Output, Self::Error> {
        let start_time = Instant::now();

        // Validate command permission
        self.check_permission(&args.command).await?;

        // Determine execution mode
        let config = SHELL_CONFIG.read().await;
        let execution_mode = args
            .execution_mode
            .clone()
            .unwrap_or_else(|| config.default_execution_mode.clone());
        drop(config);

        // Execute command based on mode
        let (stdout, stderr, exit_code) = match execution_mode {
            ShellExecutionMode::Docker => {
                tracing::info!("Executing command in Docker sandbox: {}", args.command);
                self.execute_in_docker(&args.command, args.timeout_secs).await?
            }
            ShellExecutionMode::Host => {
                tracing::warn!("Executing command on host machine: {}", args.command);
                self.execute_on_host(&args.command, args.cwd.as_deref(), args.timeout_secs).await?
            }
        };

        // Truncate output if needed
        let max_chars = crate::get_tool_execution_config().max_output_chars;

        let mut stdout = stdout;
        if stdout.len() > max_chars {
            let original_len = stdout.len();
            stdout = stdout.chars().take(max_chars).collect();
            stdout.push_str(&format!("\n... [Truncated: {}/{} chars]", stdout.len(), original_len));
        }

        let mut stderr = stderr;
        if stderr.len() > max_chars {
            let original_len = stderr.len();
            stderr = stderr.chars().take(max_chars).collect();
            stderr.push_str(&format!("\n... [Truncated: {}/{} chars]", stderr.len(), original_len));
        }

        let success = exit_code == 0;
        let execution_time_ms = start_time.elapsed().as_millis() as u64;

        Ok(ShellOutput {
            command: args.command,
            stdout,
            stderr,
            exit_code: Some(exit_code),
            success,
            execution_time_ms,
        })
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
