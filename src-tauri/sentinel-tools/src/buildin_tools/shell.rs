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
    /// Indicates if output was stored to file
    #[serde(default)]
    pub output_stored: bool,
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
    pub const DESCRIPTION: &'static str = "Execute one-time shell commands and get immediate results (e.g., ls, cat, grep, curl). For interactive tools that require continuous input/output (like msfconsole, sqlmap, database clients, Python REPL), use interactive_shell instead.";

    /// Check if command is reading files from /workspace/context/ to avoid recursive storage
    fn is_reading_context_file(command: &str) -> bool {
        let cmd = command.to_lowercase();
        let context_patterns = [
            "/workspace/context/",
            "workspace/context/",  // relative path
        ];
        
        // Check if command contains context directory path
        let has_context_path = context_patterns.iter().any(|p| cmd.contains(p));
        if !has_context_path {
            return false;
        }
        
        // Check if it's a read operation (cat, grep, tail, head, less, more, etc.)
        let read_commands = ["cat ", "grep ", "tail ", "head ", "less ", "more ", "view ", "bat "];
        read_commands.iter().any(|c| cmd.starts_with(c) || cmd.contains(&format!(" | {}", c)))
    }

    /// Execute command in Docker sandbox
    async fn execute_in_docker(
        &self,
        cmd: &str,
        timeout_secs: u64,
    ) -> Result<(String, String, i32, Arc<DockerSandbox>), ShellError> {
        // Check Docker availability first
        if !DockerSandbox::is_docker_available().await {
            return Err(ShellError::DockerError(
                "Docker is not available on this system".to_string()
            ));
        }

        let config = SHELL_CONFIG.read().await;
        let docker_config = config
            .docker_config
            .clone()
            .unwrap_or_default();
        drop(config);

        let sandbox = Arc::new(DockerSandbox::new(docker_config));
        let (stdout, stderr, exit_code) = sandbox
            .execute(cmd, timeout_secs)
            .await
            .map_err(|e| ShellError::DockerError(e.to_string()))?;
        
        Ok((stdout, stderr, exit_code, sandbox))
    }

    /// Adapt command for cross-platform execution
    fn adapt_command_for_platform(cmd: &str) -> String {
        #[cfg(target_os = "windows")]
        {
            // Convert Unix paths to Windows paths
            let adapted = cmd.replace("/workspace", "C:\\workspace")
                .replace('/', "\\");
            adapted
        }
        #[cfg(not(target_os = "windows"))]
        {
            cmd.to_string()
        }
    }

    /// Execute command on host machine with cross-platform support
    async fn execute_on_host(
        &self,
        cmd: &str,
        cwd: Option<&str>,
        timeout_secs: u64,
    ) -> Result<(String, String, i32), ShellError> {
        // Determine shell and command structure based on OS
        #[cfg(target_os = "windows")]
        let (shell, shell_arg) = {
            // Check if PowerShell is available (preferred on Windows)
            let ps_check = std::process::Command::new("powershell")
                .arg("-Command")
                .arg("$PSVersionTable.PSVersion.Major")
                .output();
            
            if ps_check.is_ok() {
                ("powershell", "-Command")
            } else {
                ("cmd", "/C")
            }
        };
        
        #[cfg(target_os = "macos")]
        let (shell, shell_arg) = {
            // macOS: prefer zsh (default since Catalina), fallback to bash
            if std::path::Path::new("/bin/zsh").exists() {
                ("/bin/zsh", "-c")
            } else {
                ("/bin/bash", "-c")
            }
        };
        
        #[cfg(all(unix, not(target_os = "macos")))]
        let (shell, shell_arg) = {
            // Linux/Unix: check available shells
            if std::path::Path::new("/bin/bash").exists() {
                ("/bin/bash", "-c")
            } else if std::path::Path::new("/bin/sh").exists() {
                ("/bin/sh", "-c")
            } else {
                ("sh", "-c")
            }
        };

        // Adapt command for platform
        let adapted_cmd = Self::adapt_command_for_platform(cmd);

        // Build command
        let mut command = Command::new(shell);
        command.arg(shell_arg);
        command.arg(&adapted_cmd);
        command.stdout(Stdio::piped());
        command.stderr(Stdio::piped());

        // Set environment variables for better compatibility
        #[cfg(target_os = "windows")]
        {
            command.env("LANG", "en_US.UTF-8");
            command.env("CHCP", "65001"); // UTF-8 code page
        }
        #[cfg(not(target_os = "windows"))]
        {
            command.env("LANG", "C.UTF-8");
            command.env("LC_ALL", "C.UTF-8");
        }

        if let Some(cwd) = cwd {
            command.current_dir(cwd);
        }

        tracing::debug!(
            "Executing on host - shell: {}, command: {}, cwd: {:?}",
            shell,
            adapted_cmd,
            cwd
        );

        // Execute with timeout
        let timeout_duration = Duration::from_secs(timeout_secs);
        let result = timeout(timeout_duration, command.output()).await;

        match result {
            Ok(Ok(output)) => {
                let stdout = String::from_utf8_lossy(&output.stdout).to_string();
                let stderr = String::from_utf8_lossy(&output.stderr).to_string();
                let exit_code = output.status.code().unwrap_or(-1);
                
                tracing::debug!(
                    "Command completed - exit_code: {}, stdout_len: {}, stderr_len: {}",
                    exit_code,
                    stdout.len(),
                    stderr.len()
                );
                
                Ok((stdout, stderr, exit_code))
            }
            Ok(Err(e)) => {
                tracing::error!("Command execution failed: {}", e);
                Err(ShellError::ExecutionFailed(e.to_string()))
            }
            Err(_) => {
                tracing::error!("Command timeout after {} seconds", timeout_secs);
                Err(ShellError::Timeout(timeout_secs))
            }
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

        // Execute command based on mode with fallback
        let (stdout, stderr, exit_code, output_stored) = match execution_mode {
            ShellExecutionMode::Docker => {
                tracing::info!("Attempting to execute command in Docker sandbox: {}", args.command);
                
                // Try Docker execution, fallback to host if Docker is unavailable
                match self.execute_in_docker(&args.command, args.timeout_secs).await {
                    Ok((stdout, stderr, exit_code, sandbox)) => {
                        // Docker execution successful
                        tracing::info!("Command executed successfully in Docker sandbox");
                        
                        // Check if command is reading context files to avoid recursive storage
                        let is_reading_context = Self::is_reading_context_file(&args.command);
                        
                        // Check if output should be stored in container
                        let storage_threshold = crate::output_storage::get_storage_threshold();
                        let mut stored = false;
                        let mut final_stdout = stdout.clone();
                        let mut final_stderr = stderr.clone();
                        
                        // Store stdout if large (unless reading context files to avoid recursion)
                        if stdout.len() > storage_threshold && !is_reading_context {
                            match crate::output_storage::store_output_in_container(
                                &sandbox,
                                "shell_stdout",
                                &stdout,
                                None,
                            ).await {
                                Ok(storage_result) => {
                                    final_stdout = storage_result.get_agent_content();
                                    stored = true;
                                }
                                Err(e) => {
                                    tracing::warn!("Failed to store stdout to container: {}", e);
                                    // Fallback to truncation
                                    final_stdout = stdout.chars().take(storage_threshold).collect();
                                    final_stdout.push_str(&format!("\n... [Truncated: {}/{} chars]", storage_threshold, stdout.len()));
                                }
                            }
                        } else if stdout.len() > storage_threshold && is_reading_context {
                            // Directly truncate to avoid recursive storage
                            tracing::info!("Command is reading context file, truncating output instead of storing");
                            final_stdout = stdout.chars().take(storage_threshold).collect();
                            final_stdout.push_str(&format!("\n... [Truncated: {}/{} chars | Reading context file]", storage_threshold, stdout.len()));
                        }
                        
                        // Store stderr if large
                        if stderr.len() > storage_threshold {
                            match crate::output_storage::store_output_in_container(
                                &sandbox,
                                "shell_stderr",
                                &stderr,
                                None,
                            ).await {
                                Ok(storage_result) => {
                                    final_stderr = storage_result.get_agent_content();
                                    stored = true;
                                }
                                Err(e) => {
                                    tracing::warn!("Failed to store stderr to container: {}", e);
                                    // Fallback to truncation
                                    final_stderr = stderr.chars().take(storage_threshold).collect();
                                    final_stderr.push_str(&format!("\n... [Truncated: {}/{} chars]", storage_threshold, stderr.len()));
                                }
                            }
                        }
                        
                        (final_stdout, final_stderr, exit_code, stored)
                    }
                    Err(e) => {
                        // Docker execution failed, fallback to host
                        tracing::warn!("Docker execution failed ({}), falling back to host execution", e);
                        tracing::warn!("Executing command on host machine: {}", args.command);
                        
                        let (stdout, stderr, exit_code) = self.execute_on_host(&args.command, args.cwd.as_deref(), args.timeout_secs).await?;
                        
                        // For host execution, use unified storage for large outputs
                        let storage_threshold = crate::output_storage::get_storage_threshold();
                        let mut stored = false;
                        let mut final_stdout = stdout.clone();
                        let mut final_stderr = stderr.clone();
                        
                        // Store stdout if large
                        if stdout.len() > storage_threshold {
                            match crate::output_storage::store_output_unified(
                                "shell_stdout_host_fallback",
                                &stdout,
                                None,
                            ).await {
                                Ok(storage_result) => {
                                    final_stdout = storage_result.get_agent_content();
                                    stored = true;
                                }
                                Err(e) => {
                                    tracing::warn!("Failed to store stdout: {}", e);
                                    final_stdout = stdout.chars().take(storage_threshold).collect();
                                    final_stdout.push_str(&format!("\n... [Truncated: {}/{} chars]", storage_threshold, stdout.len()));
                                }
                            }
                        }
                        
                        // Store stderr if large
                        if stderr.len() > storage_threshold {
                            match crate::output_storage::store_output_unified(
                                "shell_stderr_host_fallback",
                                &stderr,
                                None,
                            ).await {
                                Ok(storage_result) => {
                                    final_stderr = storage_result.get_agent_content();
                                    stored = true;
                                }
                                Err(e) => {
                                    tracing::warn!("Failed to store stderr: {}", e);
                                    final_stderr = stderr.chars().take(storage_threshold).collect();
                                    final_stderr.push_str(&format!("\n... [Truncated: {}/{} chars]", storage_threshold, stderr.len()));
                                }
                            }
                        }
                        
                        (final_stdout, final_stderr, exit_code, stored)
                    }
                }
            }
            ShellExecutionMode::Host => {
                tracing::warn!("Executing command on host machine: {}", args.command);
                let (stdout, stderr, exit_code) = self.execute_on_host(&args.command, args.cwd.as_deref(), args.timeout_secs).await?;
                
                // For host execution, also use container storage for large outputs (unified management)
                let storage_threshold = crate::output_storage::get_storage_threshold();
                let mut stored = false;
                let mut final_stdout = stdout.clone();
                let mut final_stderr = stderr.clone();
                
                // Store stdout if large
                if stdout.len() > storage_threshold {
                    match crate::output_storage::store_output_unified(
                        "shell_stdout_host",
                        &stdout,
                        None,
                    ).await {
                        Ok(storage_result) => {
                            final_stdout = storage_result.get_agent_content();
                            stored = true;
                        }
                        Err(e) => {
                            tracing::warn!("Failed to store stdout to container: {}", e);
                            // Fallback to truncation
                            final_stdout = stdout.chars().take(storage_threshold).collect();
                            final_stdout.push_str(&format!("\n... [Truncated: {}/{} chars]", storage_threshold, stdout.len()));
                        }
                    }
                }
                
                // Store stderr if large
                if stderr.len() > storage_threshold {
                    match crate::output_storage::store_output_unified(
                        "shell_stderr_host",
                        &stderr,
                        None,
                    ).await {
                        Ok(storage_result) => {
                            final_stderr = storage_result.get_agent_content();
                            stored = true;
                        }
                        Err(e) => {
                            tracing::warn!("Failed to store stderr to container: {}", e);
                            // Fallback to truncation
                            final_stderr = stderr.chars().take(storage_threshold).collect();
                            final_stderr.push_str(&format!("\n... [Truncated: {}/{} chars]", storage_threshold, stderr.len()));
                        }
                    }
                }
                
                (final_stdout, final_stderr, exit_code, stored)
            }
        };

        let success = exit_code == 0;
        let execution_time_ms = start_time.elapsed().as_millis() as u64;

        Ok(ShellOutput {
            command: args.command,
            stdout,
            stderr,
            exit_code: Some(exit_code),
            success,
            execution_time_ms,
            output_stored,
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
