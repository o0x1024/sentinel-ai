//! Shell command execution tool using rig-core Tool trait

use rig::tool::Tool;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use std::process::Stdio;
use std::time::Instant;
use tokio::process::Command;
use tokio::time::{timeout, Duration};

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
}

/// Shell command tool
#[derive(Debug, Clone)]
pub struct ShellTool {
    /// Allowed command prefixes (for security)
    allowed_prefixes: Vec<String>,
    /// Denied commands (for security)
    denied_commands: Vec<String>,
}

impl Default for ShellTool {
    fn default() -> Self {
        Self::new()
    }
}

impl ShellTool {
    pub fn new() -> Self {
        Self {
            allowed_prefixes: vec![],
            denied_commands: vec![
                "rm -rf /".to_string(),
                "mkfs".to_string(),
                "dd if=/dev/zero".to_string(),
            ],
        }
    }

    /// Create with allowed command prefixes
    pub fn with_allowed_prefixes(prefixes: Vec<String>) -> Self {
        Self {
            allowed_prefixes: prefixes,
            denied_commands: vec![
                "rm -rf /".to_string(),
                "mkfs".to_string(),
                "dd if=/dev/zero".to_string(),
            ],
        }
    }

    /// Validate command for security
    fn validate_command(&self, cmd: &str) -> Result<(), ShellError> {
        let cmd_lower = cmd.to_lowercase();

        // Check denied commands
        for denied in &self.denied_commands {
            if cmd_lower.contains(&denied.to_lowercase()) {
                return Err(ShellError::InvalidCommand(
                    format!("Command contains denied pattern: {}", denied)
                ));
            }
        }

        // Check allowed prefixes if configured
        if !self.allowed_prefixes.is_empty() {
            let allowed = self.allowed_prefixes.iter().any(|prefix| {
                cmd.starts_with(prefix) || cmd_lower.starts_with(&prefix.to_lowercase())
            });
            if !allowed {
                return Err(ShellError::InvalidCommand(
                    "Command not in allowed list".to_string()
                ));
            }
        }

        Ok(())
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
            description: "Execute shell commands. Use with caution - some commands are restricted for security.".to_string(),
            parameters: serde_json::to_value(schemars::schema_for!(ShellArgs))
                .unwrap_or_default(),
        }
    }

    async fn call(&self, args: Self::Args) -> Result<Self::Output, Self::Error> {
        let start_time = Instant::now();

        // Validate command
        self.validate_command(&args.command)?;

        // Determine shell
        #[cfg(target_os = "windows")]
        let (shell, shell_arg) = ("cmd", "/C");
        #[cfg(not(target_os = "windows"))]
        let (shell, shell_arg) = ("sh", "-c");

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

    #[test]
    fn test_validate_command() {
        let tool = ShellTool::new();
        
        // Denied commands should fail
        assert!(tool.validate_command("rm -rf /").is_err());
        
        // Normal commands should pass
        assert!(tool.validate_command("ls -la").is_ok());
        assert!(tool.validate_command("echo hello").is_ok());
    }

    #[test]
    fn test_allowed_prefixes() {
        let tool = ShellTool::with_allowed_prefixes(vec!["ls".to_string(), "echo".to_string()]);
        
        assert!(tool.validate_command("ls -la").is_ok());
        assert!(tool.validate_command("echo hello").is_ok());
        assert!(tool.validate_command("cat /etc/passwd").is_err());
    }
}

