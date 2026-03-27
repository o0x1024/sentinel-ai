//! Shell command execution tool using rig-core Tool trait

use crate::docker_sandbox::{DockerSandbox, DockerSandboxConfig};
use once_cell::sync::Lazy;
use rig::tool::Tool;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::process::Stdio;
use std::sync::Arc;
use std::time::Instant;
use tokio::process::Command;
use tokio::sync::RwLock;
use tokio::time::Duration;
use tokio_util::sync::CancellationToken;

/// Shell execution mode
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, JsonSchema, Default)]
pub enum ShellExecutionMode {
    /// Execute on host machine (less secure)
    Host,
    /// Execute in Docker container (more secure)
    #[default]
    Docker,
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
    /// Internal execution id for cancellation scope
    #[serde(default)]
    pub execution_id: Option<String>,
    /// Whether to store oversized output into context files (agent-only)
    #[serde(default)]
    pub enable_large_output_storage: bool,
}

fn default_timeout() -> u64 {
    180
}

/// Shell command result
#[derive(Debug, Clone, Serialize)]
pub struct ShellOutput {
    pub command: String,
    pub stdout: String,
    pub stderr: String,
    pub exit_code: Option<i32>,
    pub completed: bool,
    pub execution_time_ms: u64,
    /// Indicates if output was stored to file
    #[serde(default)]
    pub output_stored: bool,
    /// Actual execution mode used: "docker" | "host"
    #[serde(default)]
    pub execution_mode: String,
    /// If fallback happened, indicates original mode
    #[serde(default)]
    pub fallback_from: Option<String>,
    /// If fallback happened, includes failure reason
    #[serde(default)]
    pub fallback_reason: Option<String>,
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
    #[error("Shell execution cancelled")]
    Cancelled,
}

/// Shell default policy
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, JsonSchema, Default)]
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
    async fn check_permission(&self, command: &str, execution_id: Option<&str>) -> bool;
}

/// Global shell configuration
static SHELL_CONFIG: Lazy<RwLock<ShellConfig>> = Lazy::new(|| RwLock::new(ShellConfig::default()));

/// Global permission handler
static PERMISSION_HANDLER: Lazy<RwLock<Option<Arc<dyn ShellPermissionHandler>>>> =
    Lazy::new(|| RwLock::new(None));
/// Per-execution shell cancellation tokens
static SHELL_CANCELLATION_TOKENS: Lazy<RwLock<HashMap<String, CancellationToken>>> =
    Lazy::new(|| RwLock::new(HashMap::new()));

/// Register/refresh shell cancellation token for an execution
pub async fn register_shell_execution_cancellation(execution_id: &str) -> CancellationToken {
    let token = CancellationToken::new();
    let mut map = SHELL_CANCELLATION_TOKENS.write().await;
    map.insert(execution_id.to_string(), token.clone());
    token
}

/// Cancel active shell execution for a specific execution id
pub async fn cancel_shell_execution(execution_id: &str) -> bool {
    let map = SHELL_CANCELLATION_TOKENS.read().await;
    if let Some(token) = map.get(execution_id) {
        token.cancel();
        true
    } else {
        false
    }
}

/// Clear shell cancellation token after command completion
pub async fn clear_shell_execution_cancellation(execution_id: &str) {
    let mut map = SHELL_CANCELLATION_TOKENS.write().await;
    map.remove(execution_id);
}

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

/// Check command permission using current shell config
pub async fn check_shell_permission(
    command: &str,
    execution_id: Option<&str>,
) -> Result<(), ShellError> {
    let config = SHELL_CONFIG.read().await;
    check_shell_permission_with_config(command, &config, execution_id).await
}

async fn check_shell_permission_with_config(
    command: &str,
    config: &ShellConfig,
    execution_id: Option<&str>,
) -> Result<(), ShellError> {
    // Check if command is in deny list (always deny these)
    if config.is_denied(command) {
        return Err(ShellError::PermissionDenied(format!(
            "Command denied by policy: {}",
            command
        )));
    }

    // Check if command is auto-allowed
    if config.is_allowed(command) {
        return Ok(());
    }

    // Check if needs confirmation based on policy
    if config.needs_confirmation(command) {
        return ask_permission(command, execution_id).await;
    }

    Ok(())
}

async fn ask_permission(command: &str, execution_id: Option<&str>) -> Result<(), ShellError> {
    let handler_guard = PERMISSION_HANDLER.read().await;
    if let Some(handler) = &*handler_guard {
        if handler.check_permission(command, execution_id).await {
            Ok(())
        } else {
            Err(ShellError::PermissionDenied(
                "User rejected execution".to_string(),
            ))
        }
    } else {
        // If no handler is registered, deny by default for safety
        Err(ShellError::PermissionDenied(
            "No permission handler registered to ask user".to_string(),
        ))
    }
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
    pub const DESCRIPTION: &'static str = "Execute one-time shell commands and get immediate results (e.g., ls, cat, grep, curl). For interactive tools, long-lived services, or background commands that keep running (like msfconsole, sqlmap, database clients, Python REPL, dev servers), use interactive_shell instead.";

    /// Check if command is reading files from /workspace/context/ to avoid recursive storage
    fn is_reading_context_file(command: &str) -> bool {
        let cmd = command.to_lowercase();
        let context_patterns = [
            "/workspace/context/",
            "workspace/context/", // relative path
        ];

        // Check if command contains context directory path
        let has_context_path = context_patterns.iter().any(|p| cmd.contains(p));
        if !has_context_path {
            return false;
        }

        // Check if it's a read operation (cat, grep, tail, head, less, more, etc.)
        let read_commands = [
            "cat ", "grep ", "tail ", "head ", "less ", "more ", "view ", "bat ",
        ];
        read_commands
            .iter()
            .any(|c| cmd.starts_with(c) || cmd.contains(&format!(" | {}", c)))
    }

    fn scan_unquoted_shell(command: &str, mut f: impl FnMut(char, Option<char>, Option<char>)) {
        let chars: Vec<char> = command.chars().collect();
        let mut in_single = false;
        let mut in_double = false;
        let mut escaped = false;

        for (idx, ch) in chars.iter().copied().enumerate() {
            if escaped {
                escaped = false;
                continue;
            }

            if ch == '\\' && !in_single {
                escaped = true;
                continue;
            }

            if ch == '\'' && !in_double {
                in_single = !in_single;
                continue;
            }

            if ch == '"' && !in_single {
                in_double = !in_double;
                continue;
            }

            if in_single || in_double {
                continue;
            }

            let prev = idx.checked_sub(1).and_then(|i| chars.get(i)).copied();
            let next = chars.get(idx + 1).copied();
            f(ch, prev, next);
        }
    }

    fn has_background_operator(command: &str) -> bool {
        let mut found = false;
        Self::scan_unquoted_shell(command, |ch, prev, next| {
            if found || ch != '&' {
                return;
            }
            if prev == Some('>') || next == Some('>') {
                return;
            }
            if prev == Some('&') || next == Some('&') {
                return;
            }
            found = true;
        });
        found
    }

    fn background_command_keeps_stdio_attached(command: &str) -> bool {
        if !Self::has_background_operator(command) {
            return false;
        }

        let mut stdout_redirected = false;
        let mut stderr_redirected = false;
        let mut stderr_to_stdout = false;

        let chars: Vec<char> = command.chars().collect();
        Self::scan_unquoted_shell(command, |ch, prev, next| {
            if ch == '>' {
                match prev {
                    Some('&') => {
                        stdout_redirected = true;
                        stderr_redirected = true;
                    }
                    Some('2') => {
                        stderr_redirected = true;
                    }
                    _ => {
                        stdout_redirected = true;
                    }
                }
            }

            if ch == '2' && next == Some('>') {
                stderr_redirected = true;
            }

            if ch == '2'
                && chars.windows(4).any(|window| window == ['2', '>', '&', '1'])
            {
                stderr_to_stdout = true;
            }
        });

        let stderr_detached = stderr_redirected || (stderr_to_stdout && stdout_redirected);
        !(stdout_redirected && stderr_detached)
    }

    fn build_background_command_guidance(command: &str) -> Option<String> {
        if !Self::background_command_keeps_stdio_attached(command) {
            return None;
        }

        Some(
            "Detected a background shell command that keeps stdout/stderr attached. The one-shot \
shell tool waits for those pipes to close, so this command would stay in running state. Use \
interactive_shell for long-lived processes, or fully detach the command, for example: \
`nohup <command> >/tmp/sentinel-shell.log 2>&1 < /dev/null & echo $!`."
                .to_string(),
        )
    }

    /// Execute command in Docker sandbox
    async fn execute_in_docker(
        &self,
        cmd: &str,
        timeout_secs: u64,
        cancellation_token: Option<CancellationToken>,
    ) -> Result<(String, String, i32, Arc<DockerSandbox>), ShellError> {
        // Check Docker availability first
        if !DockerSandbox::is_docker_available().await {
            return Err(ShellError::DockerError(
                "Docker is not available on this system".to_string(),
            ));
        }

        let config = SHELL_CONFIG.read().await;
        let docker_config = config.docker_config.clone().unwrap_or_default();
        drop(config);

        let sandbox = Arc::new(DockerSandbox::new(docker_config));
        let (stdout, stderr, exit_code) = sandbox
            .execute_with_cancellation(cmd, timeout_secs, cancellation_token)
            .await
            .map_err(|e| match e {
                crate::docker_sandbox::DockerError::Timeout(secs) => ShellError::Timeout(secs),
                crate::docker_sandbox::DockerError::ExecutionFailed(msg) if msg == "cancelled" => {
                    ShellError::Cancelled
                }
                _ => ShellError::DockerError(e.to_string()),
            })?;

        Ok((stdout, stderr, exit_code, sandbox))
    }

    /// Adapt command for cross-platform execution
    fn adapt_command_for_platform(cmd: &str) -> String {
        #[cfg(target_os = "windows")]
        {
            // Convert Unix paths to Windows paths
            let adapted = cmd
                .replace("/workspace", "C:\\workspace")
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
        cancellation_token: Option<CancellationToken>,
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

        let mut child = command
            .spawn()
            .map_err(|e| ShellError::ExecutionFailed(e.to_string()))?;
        let timeout_duration = tokio::time::sleep(Duration::from_secs(timeout_secs));
        tokio::pin!(timeout_duration);

        loop {
            if let Some(token) = cancellation_token.as_ref() {
                if token.is_cancelled() {
                    let _ = child.kill().await;
                    let _ = child.wait().await;
                    return Err(ShellError::Cancelled);
                }
            }

            match child.try_wait() {
                Ok(Some(_status)) => {
                    let output = child
                        .wait_with_output()
                        .await
                        .map_err(|e| ShellError::ExecutionFailed(e.to_string()))?;

                    let stdout = String::from_utf8_lossy(&output.stdout).to_string();
                    let stderr = String::from_utf8_lossy(&output.stderr).to_string();
                    let exit_code = output.status.code().unwrap_or(-1);

                    tracing::debug!(
                        "Command completed - exit_code: {}, stdout_len: {}, stderr_len: {}",
                        exit_code,
                        stdout.len(),
                        stderr.len()
                    );

                    return Ok((stdout, stderr, exit_code));
                }
                Ok(None) => {
                    tokio::select! {
                        _ = &mut timeout_duration => {
                            let _ = child.kill().await;
                            let _ = child.wait().await;
                            tracing::error!("Command timeout after {} seconds", timeout_secs);
                            return Err(ShellError::Timeout(timeout_secs));
                        }
                        _ = tokio::time::sleep(Duration::from_millis(120)) => {}
                    }
                }
                Err(e) => {
                    tracing::error!("Command execution failed: {}", e);
                    return Err(ShellError::ExecutionFailed(e.to_string()));
                }
            }
        }
    }

    /// Validate command permissions
    async fn check_permission(
        &self,
        cmd: &str,
        execution_id: Option<&str>,
    ) -> Result<(), ShellError> {
        check_shell_permission(cmd, execution_id).await
    }
}

impl Tool for ShellTool {
    const NAME: &'static str = Self::NAME;
    type Args = ShellArgs;
    type Output = ShellOutput;
    type Error = ShellError;

    async fn definition(&self, _prompt: String) -> rig::completion::ToolDefinition {
        let config = SHELL_CONFIG.read().await;
        let is_docker = config.default_execution_mode == ShellExecutionMode::Docker;

        let mut desc = Self::DESCRIPTION.to_string();
        if is_docker {
            desc.push_str(" [ENVIRONMENT: This shell runs in a Kali Linux docker sandbox with pre-installed cybersecurity tools like nmap, sqlmap, msfconsole, masscan, dirb, etc. Do not hesitate to use these tools directly.]");
        } else {
            desc.push_str(" [ENVIRONMENT: This shell runs on the Host OS.]");
        }

        rig::completion::ToolDefinition {
            name: Self::NAME.to_string(),
            description: desc,
            parameters: serde_json::to_value(schemars::schema_for!(ShellArgs)).unwrap_or_default(),
        }
    }

    async fn call(&self, args: Self::Args) -> Result<Self::Output, Self::Error> {
        let start_time = Instant::now();
        if let Some(guidance) = Self::build_background_command_guidance(&args.command) {
            return Err(ShellError::ExecutionFailed(guidance));
        }
        let execution_id = args.execution_id.clone();
        let cancellation_token = if let Some(exec_id) = execution_id.as_deref() {
            Some(register_shell_execution_cancellation(exec_id).await)
        } else {
            None
        };

        // Determine execution mode
        let config = SHELL_CONFIG.read().await;
        let execution_mode = args
            .execution_mode
            .clone()
            .unwrap_or_else(|| config.default_execution_mode.clone());
        drop(config);

        // Execute command based on mode with fallback
        let execution_result: Result<
            (
                String,
                String,
                i32,
                bool,
                ShellExecutionMode,
                Option<String>,
            ),
            ShellError,
        > = (async {
            let enable_large_output_storage = args.enable_large_output_storage;
            match execution_mode {
            ShellExecutionMode::Docker => {
                tracing::info!("Attempting to execute command in Docker sandbox: {}", args.command);
                
                // Try Docker execution, fallback to host if Docker is unavailable
                match self.execute_in_docker(&args.command, args.timeout_secs, cancellation_token.clone()).await {
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
                        if enable_large_output_storage && stdout.len() > storage_threshold && !is_reading_context {
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
                        } else if enable_large_output_storage && stdout.len() > storage_threshold && is_reading_context {
                            // Directly truncate to avoid recursive storage
                            tracing::info!("Command is reading context file, truncating output instead of storing");
                            final_stdout = stdout.chars().take(storage_threshold).collect();
                            final_stdout.push_str(&format!("\n... [Truncated: {}/{} chars | Reading context file]", storage_threshold, stdout.len()));
                        }
                        
                        // Store stderr if large
                        if enable_large_output_storage && stderr.len() > storage_threshold {
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
                        
                        Ok((
                            final_stdout,
                            final_stderr,
                            exit_code,
                            stored,
                            ShellExecutionMode::Docker,
                            None,
                        ))
                    }
                    Err(e) => {
                        if matches!(e, ShellError::Timeout(_) | ShellError::Cancelled) {
                            Err(e)
                        } else {
                            // Docker execution failed, fallback to host
                            tracing::warn!("Docker execution failed ({}), falling back to host execution", e);
                            tracing::warn!("Executing command on host machine: {}", args.command);

                            // Host execution requires permission check
                            self.check_permission(&args.command, execution_id.as_deref()).await?;
                            
                            let (stdout, stderr, exit_code) = self.execute_on_host(
                                &args.command,
                                args.cwd.as_deref(),
                                args.timeout_secs,
                                cancellation_token.clone(),
                            ).await?;
                            
                            // For host execution fallback, force host storage for clear path semantics
                            let storage_threshold = crate::output_storage::get_storage_threshold();
                            let mut stored = false;
                            let mut final_stdout = stdout.clone();
                            let mut final_stderr = stderr.clone();
                            
                            // Store stdout if large
                            if enable_large_output_storage && stdout.len() > storage_threshold {
                                match crate::output_storage::store_output_on_host(
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
                            if enable_large_output_storage && stderr.len() > storage_threshold {
                                match crate::output_storage::store_output_on_host(
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
                            
                            Ok((
                                final_stdout,
                                final_stderr,
                                exit_code,
                                stored,
                                ShellExecutionMode::Host,
                                Some(e.to_string()),
                            ))
                        }
                    }
                }
            }
            ShellExecutionMode::Host => {
                tracing::warn!("Executing command on host machine: {}", args.command);
                self.check_permission(&args.command, execution_id.as_deref()).await?;
                let (stdout, stderr, exit_code) = self.execute_on_host(
                    &args.command,
                    args.cwd.as_deref(),
                    args.timeout_secs,
                    cancellation_token.clone(),
                ).await?;
                
                // In explicit host mode, always store outputs on host so displayed paths match mode.
                let storage_threshold = crate::output_storage::get_storage_threshold();
                let mut stored = false;
                let mut final_stdout = stdout.clone();
                let mut final_stderr = stderr.clone();
                
                // Store stdout if large
                if enable_large_output_storage && stdout.len() > storage_threshold {
                    match crate::output_storage::store_output_on_host(
                        "shell_stdout_host",
                        &stdout,
                        None,
                    ).await {
                        Ok(storage_result) => {
                            final_stdout = storage_result.get_agent_content();
                            stored = true;
                        }
                        Err(e) => {
                            tracing::warn!("Failed to store stdout: {}", e);
                            // Fallback to truncation
                            final_stdout = stdout.chars().take(storage_threshold).collect();
                            final_stdout.push_str(&format!("\n... [Truncated: {}/{} chars]", storage_threshold, stdout.len()));
                        }
                    }
                }
                
                // Store stderr if large
                if enable_large_output_storage && stderr.len() > storage_threshold {
                    match crate::output_storage::store_output_on_host(
                        "shell_stderr_host",
                        &stderr,
                        None,
                    ).await {
                        Ok(storage_result) => {
                            final_stderr = storage_result.get_agent_content();
                            stored = true;
                        }
                        Err(e) => {
                            tracing::warn!("Failed to store stderr: {}", e);
                            // Fallback to truncation
                            final_stderr = stderr.chars().take(storage_threshold).collect();
                            final_stderr.push_str(&format!("\n... [Truncated: {}/{} chars]", storage_threshold, stderr.len()));
                        }
                    }
                }
                
                Ok((
                    final_stdout,
                    final_stderr,
                    exit_code,
                    stored,
                    ShellExecutionMode::Host,
                    None,
                ))
            }
            }
        }).await;
        if let Some(exec_id) = execution_id.as_deref() {
            clear_shell_execution_cancellation(exec_id).await;
        }
        let (stdout, stderr, exit_code, output_stored, actual_mode, fallback_reason) =
            execution_result?;

        let execution_time_ms = start_time.elapsed().as_millis() as u64;
        let execution_mode = match actual_mode {
            ShellExecutionMode::Docker => "docker".to_string(),
            ShellExecutionMode::Host => "host".to_string(),
        };
        let fallback_from = if fallback_reason.is_some() {
            Some("docker".to_string())
        } else {
            None
        };

        Ok(ShellOutput {
            command: args.command,
            stdout,
            stderr,
            exit_code: Some(exit_code),
            completed: true,
            execution_time_ms,
            output_stored,
            execution_mode,
            fallback_from,
            fallback_reason,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_detects_background_operator() {
        assert!(ShellTool::has_background_operator(
            "python3 -m http.server 8000 &"
        ));
        assert!(!ShellTool::has_background_operator("echo foo && echo bar"));
        assert!(!ShellTool::has_background_operator("echo '&'"));
    }

    #[test]
    fn test_detects_background_command_with_attached_stdio() {
        assert!(ShellTool::background_command_keeps_stdio_attached(
            "python3 -m http.server 8000 &"
        ));
        assert!(ShellTool::build_background_command_guidance(
            "python3 -m http.server 8000 &"
        )
        .is_some());
    }

    #[test]
    fn test_allows_detached_background_command() {
        assert!(!ShellTool::background_command_keeps_stdio_attached(
            "nohup python3 -m http.server 8000 >/tmp/http.log 2>&1 < /dev/null & echo $!"
        ));
        assert!(ShellTool::build_background_command_guidance(
            "nohup python3 -m http.server 8000 >/tmp/http.log 2>&1 < /dev/null & echo $!"
        )
        .is_none());
    }

    #[tokio::test]
    async fn test_default_rules() {
        let tool = ShellTool::new();

        // Denied by default rule
        assert!(matches!(
            tool.check_permission("rm -rf /", None).await,
            Err(ShellError::PermissionDenied(_))
        ));
    }
}
