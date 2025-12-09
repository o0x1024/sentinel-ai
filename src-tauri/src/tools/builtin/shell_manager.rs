//! Shell 管理器
//!
//! 管理后台 shell 进程，支持：
//! - 前台命令执行（等待完成）
//! - 后台命令执行（立即返回）
//! - 输出流读取
//! - 进程终止

use anyhow::{anyhow, Result};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::process::Stdio;
use std::sync::Arc;
use tokio::io::{AsyncBufReadExt, BufReader};
use tokio::process::{Child, Command};
use tokio::sync::{mpsc, Mutex, RwLock};
use tokio::time::{timeout, Duration};
use uuid::Uuid;

/// Shell 状态
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum ShellStatus {
    Running,
    Completed,
    Failed,
    Killed,
}

impl std::fmt::Display for ShellStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ShellStatus::Running => write!(f, "running"),
            ShellStatus::Completed => write!(f, "completed"),
            ShellStatus::Failed => write!(f, "failed"),
            ShellStatus::Killed => write!(f, "killed"),
        }
    }
}

/// Shell 执行结果
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ShellResult {
    pub shell_id: String,
    pub status: ShellStatus,
    pub exit_code: Option<i32>,
    pub stdout: String,
    pub stderr: String,
    pub duration_ms: u64,
}

/// 后台 Shell 信息
#[derive(Debug)]
pub struct BackgroundShell {
    pub id: String,
    pub command: String,
    pub description: Option<String>,
    pub status: ShellStatus,
    pub started_at: chrono::DateTime<chrono::Utc>,
    pub output_buffer: Arc<Mutex<Vec<String>>>,
    pub error_buffer: Arc<Mutex<Vec<String>>>,
    pub exit_code: Option<i32>,
    kill_sender: Option<mpsc::Sender<()>>,
}

impl BackgroundShell {
    /// 获取新输出（自上次读取后）
    pub async fn get_new_output(&self) -> (String, String) {
        let stdout = {
            let mut buf = self.output_buffer.lock().await;
            let output = buf.join("\n");
            buf.clear();
            output
        };
        let stderr = {
            let mut buf = self.error_buffer.lock().await;
            let output = buf.join("\n");
            buf.clear();
            output
        };
        (stdout, stderr)
    }

    /// 获取所有输出（不清空缓冲区）
    pub async fn peek_output(&self) -> (String, String) {
        let stdout = self.output_buffer.lock().await.join("\n");
        let stderr = self.error_buffer.lock().await.join("\n");
        (stdout, stderr)
    }

    /// 发送终止信号
    pub async fn kill(&mut self) -> Result<()> {
        if let Some(sender) = self.kill_sender.take() {
            sender.send(()).await.ok();
            self.status = ShellStatus::Killed;
        }
        Ok(())
    }
}

/// Shell 管理器
pub struct ShellManager {
    shells: Arc<RwLock<HashMap<String, Arc<Mutex<BackgroundShell>>>>>,
    default_timeout_ms: u64,
    max_output_chars: usize,
}

impl std::fmt::Debug for ShellManager {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ShellManager")
            .field("default_timeout_ms", &self.default_timeout_ms)
            .field("max_output_chars", &self.max_output_chars)
            .finish()
    }
}

impl ShellManager {
    /// 创建新的 Shell 管理器
    pub fn new() -> Self {
        Self {
            shells: Arc::new(RwLock::new(HashMap::new())),
            default_timeout_ms: 120_000, // 2 minutes
            max_output_chars: 30_000,
        }
    }

    /// 设置默认超时
    pub fn with_timeout(mut self, timeout_ms: u64) -> Self {
        self.default_timeout_ms = timeout_ms;
        self
    }

    /// 执行前台命令（等待完成）
    pub async fn execute(
        &self,
        command: &str,
        timeout_ms: Option<u64>,
        description: Option<&str>,
    ) -> Result<ShellResult> {
        let shell_id = Uuid::new_v4().to_string();
        let timeout_duration = Duration::from_millis(timeout_ms.unwrap_or(self.default_timeout_ms));
        let start_time = std::time::Instant::now();

        tracing::info!(
            "Executing command: {} (timeout: {}ms)",
            truncate_command(command, 100),
            timeout_duration.as_millis()
        );

        let mut child = self.spawn_shell(command)?;

        let stdout_handle = child.stdout.take();
        let stderr_handle = child.stderr.take();

        // 收集输出
        let stdout_task = tokio::spawn(async move {
            if let Some(stdout) = stdout_handle {
                let reader = BufReader::new(stdout);
                let mut lines = reader.lines();
                let mut output = Vec::new();
                while let Ok(Some(line)) = lines.next_line().await {
                    output.push(line);
                }
                output.join("\n")
            } else {
                String::new()
            }
        });

        let stderr_task = tokio::spawn(async move {
            if let Some(stderr) = stderr_handle {
                let reader = BufReader::new(stderr);
                let mut lines = reader.lines();
                let mut output = Vec::new();
                while let Ok(Some(line)) = lines.next_line().await {
                    output.push(line);
                }
                output.join("\n")
            } else {
                String::new()
            }
        });

        // 等待进程完成（带超时）
        let result = timeout(timeout_duration, child.wait()).await;

        let duration_ms = start_time.elapsed().as_millis() as u64;

        let stdout = stdout_task.await.unwrap_or_default();
        let stderr = stderr_task.await.unwrap_or_default();

        match result {
            Ok(Ok(exit_status)) => {
                let exit_code = exit_status.code();
                let status = if exit_status.success() {
                    ShellStatus::Completed
                } else {
                    ShellStatus::Failed
                };

                tracing::info!(
                    "Command completed: {} (exit: {:?}, duration: {}ms)",
                    truncate_command(command, 50),
                    exit_code,
                    duration_ms
                );

                Ok(ShellResult {
                    shell_id,
                    status,
                    exit_code,
                    stdout: truncate_output(&stdout, self.max_output_chars),
                    stderr: truncate_output(&stderr, self.max_output_chars),
                    duration_ms,
                })
            }
            Ok(Err(e)) => {
                tracing::error!("Command failed to execute: {}", e);
                Ok(ShellResult {
                    shell_id,
                    status: ShellStatus::Failed,
                    exit_code: None,
                    stdout: truncate_output(&stdout, self.max_output_chars),
                    stderr: format!("Execution error: {}", e),
                    duration_ms,
                })
            }
            Err(_) => {
                tracing::warn!("Command timed out after {}ms", timeout_duration.as_millis());
                Ok(ShellResult {
                    shell_id,
                    status: ShellStatus::Failed,
                    exit_code: None,
                    stdout: truncate_output(&stdout, self.max_output_chars),
                    stderr: format!("Command timed out after {}ms", timeout_duration.as_millis()),
                    duration_ms,
                })
            }
        }
    }

    /// 在后台执行命令（立即返回）
    pub async fn execute_background(
        &self,
        command: &str,
        description: Option<&str>,
    ) -> Result<String> {
        let shell_id = Uuid::new_v4().to_string();
        let (kill_tx, mut kill_rx) = mpsc::channel(1);

        let output_buffer = Arc::new(Mutex::new(Vec::new()));
        let error_buffer = Arc::new(Mutex::new(Vec::new()));

        let shell = BackgroundShell {
            id: shell_id.clone(),
            command: command.to_string(),
            description: description.map(String::from),
            status: ShellStatus::Running,
            started_at: chrono::Utc::now(),
            output_buffer: output_buffer.clone(),
            error_buffer: error_buffer.clone(),
            exit_code: None,
            kill_sender: Some(kill_tx),
        };

        let shell = Arc::new(Mutex::new(shell));
        {
            let mut shells = self.shells.write().await;
            shells.insert(shell_id.clone(), shell.clone());
        }

        // 启动后台任务
        let mut child = self.spawn_shell(command)?;
        let stdout = child.stdout.take();
        let stderr = child.stderr.take();

        let shell_clone = shell.clone();
        let shells_ref = self.shells.clone();
        let shell_id_clone = shell_id.clone();

        tokio::spawn(async move {
            // 读取 stdout
            let output_buf = output_buffer.clone();
            let stdout_task = tokio::spawn(async move {
                if let Some(stdout) = stdout {
                    let reader = BufReader::new(stdout);
                    let mut lines = reader.lines();
                    while let Ok(Some(line)) = lines.next_line().await {
                        output_buf.lock().await.push(line);
                    }
                }
            });

            // 读取 stderr
            let error_buf = error_buffer.clone();
            let stderr_task = tokio::spawn(async move {
                if let Some(stderr) = stderr {
                    let reader = BufReader::new(stderr);
                    let mut lines = reader.lines();
                    while let Ok(Some(line)) = lines.next_line().await {
                        error_buf.lock().await.push(line);
                    }
                }
            });

            // 等待进程完成或被终止
            tokio::select! {
                status = child.wait() => {
                    let _ = stdout_task.await;
                    let _ = stderr_task.await;

                    let mut shell = shell_clone.lock().await;
                    match status {
                        Ok(exit_status) => {
                            shell.exit_code = exit_status.code();
                            shell.status = if exit_status.success() {
                                ShellStatus::Completed
                            } else {
                                ShellStatus::Failed
                            };
                        }
                        Err(_) => {
                            shell.status = ShellStatus::Failed;
                        }
                    }
                }
                _ = kill_rx.recv() => {
                    let _ = child.kill().await;
                    let mut shell = shell_clone.lock().await;
                    shell.status = ShellStatus::Killed;
                }
            }
        });

        tracing::info!("Started background shell: {}", shell_id);
        Ok(shell_id)
    }

    /// 获取后台 shell 的输出
    pub async fn get_output(&self, shell_id: &str, filter: Option<&str>) -> Result<ShellResult> {
        let shells = self.shells.read().await;
        let shell = shells
            .get(shell_id)
            .ok_or_else(|| anyhow!("Shell not found: {}", shell_id))?;

        let shell = shell.lock().await;
        let (stdout, stderr) = shell.get_new_output().await;

        // 应用过滤器
        let stdout = if let Some(pattern) = filter {
            filter_output(&stdout, pattern)
        } else {
            stdout
        };

        Ok(ShellResult {
            shell_id: shell_id.to_string(),
            status: shell.status.clone(),
            exit_code: shell.exit_code,
            stdout: truncate_output(&stdout, self.max_output_chars),
            stderr: truncate_output(&stderr, self.max_output_chars),
            duration_ms: (chrono::Utc::now() - shell.started_at).num_milliseconds() as u64,
        })
    }

    /// 终止后台 shell
    pub async fn kill(&self, shell_id: &str) -> Result<()> {
        let shells = self.shells.read().await;
        let shell = shells
            .get(shell_id)
            .ok_or_else(|| anyhow!("Shell not found: {}", shell_id))?;

        let mut shell = shell.lock().await;
        shell.kill().await?;

        tracing::info!("Killed shell: {}", shell_id);
        Ok(())
    }

    /// 列出所有后台 shell
    pub async fn list(&self) -> Vec<ShellInfo> {
        let shells = self.shells.read().await;
        let mut result = Vec::new();

        for (id, shell) in shells.iter() {
            let shell = shell.lock().await;
            result.push(ShellInfo {
                id: id.clone(),
                command: truncate_command(&shell.command, 50),
                description: shell.description.clone(),
                status: shell.status.clone(),
                started_at: shell.started_at.to_rfc3339(),
            });
        }

        result
    }

    /// 清理已完成的 shell
    pub async fn cleanup(&self) {
        let mut shells = self.shells.write().await;
        shells.retain(|_, shell| {
            let shell = shell.blocking_lock();
            shell.status == ShellStatus::Running
        });
    }

    /// 创建 shell 子进程
    fn spawn_shell(&self, command: &str) -> Result<Child> {
        #[cfg(target_os = "windows")]
        let child = Command::new("cmd")
            .args(["/C", command])
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .stdin(Stdio::null())
            .spawn()?;

        #[cfg(not(target_os = "windows"))]
        let child = Command::new("sh")
            .args(["-c", command])
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .stdin(Stdio::null())
            .spawn()?;

        Ok(child)
    }
}

impl Default for ShellManager {
    fn default() -> Self {
        Self::new()
    }
}

/// Shell 信息（用于列表展示）
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ShellInfo {
    pub id: String,
    pub command: String,
    pub description: Option<String>,
    pub status: ShellStatus,
    pub started_at: String,
}

/// 截断命令显示
fn truncate_command(cmd: &str, max_len: usize) -> String {
    if cmd.len() <= max_len {
        cmd.to_string()
    } else {
        format!("{}...", &cmd[..max_len - 3])
    }
}

/// 截断输出
fn truncate_output(output: &str, max_chars: usize) -> String {
    if output.len() <= max_chars {
        output.to_string()
    } else {
        format!(
            "{}...\n[Output truncated, {} characters total]",
            &output[..max_chars - 50],
            output.len()
        )
    }
}

/// 使用正则表达式过滤输出
fn filter_output(output: &str, pattern: &str) -> String {
    match regex::Regex::new(pattern) {
        Ok(re) => output
            .lines()
            .filter(|line| re.is_match(line))
            .collect::<Vec<_>>()
            .join("\n"),
        Err(_) => output.to_string(),
    }
}

// ============================================================================
// 全局 Shell 管理器
// ============================================================================

use once_cell::sync::OnceCell;

static GLOBAL_SHELL_MANAGER: OnceCell<Arc<ShellManager>> = OnceCell::new();

/// 初始化全局 Shell 管理器
pub fn initialize_global_shell_manager() {
    GLOBAL_SHELL_MANAGER.get_or_init(|| Arc::new(ShellManager::new()));
}

/// 获取全局 Shell 管理器
pub fn get_global_shell_manager() -> Option<Arc<ShellManager>> {
    GLOBAL_SHELL_MANAGER.get().cloned()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_execute_simple_command() {
        let manager = ShellManager::new();

        #[cfg(target_os = "windows")]
        let result = manager.execute("echo hello", None, None).await.unwrap();
        #[cfg(not(target_os = "windows"))]
        let result = manager.execute("echo hello", None, None).await.unwrap();

        assert_eq!(result.status, ShellStatus::Completed);
        assert!(result.stdout.contains("hello"));
    }

    #[tokio::test]
    async fn test_execute_with_timeout() {
        let manager = ShellManager::new();

        // 测试超时
        let result = manager.execute("sleep 10", Some(100), None).await.unwrap();
        assert_eq!(result.status, ShellStatus::Failed);
        assert!(result.stderr.contains("timed out"));
    }

    #[tokio::test]
    async fn test_background_execution() {
        let manager = ShellManager::new();

        // 启动后台进程
        let shell_id = manager
            .execute_background("sleep 1 && echo done", Some("Test sleep"))
            .await
            .unwrap();

        // 检查状态
        let list = manager.list().await;
        assert!(list.iter().any(|s| s.id == shell_id));

        // 等待完成
        tokio::time::sleep(Duration::from_secs(2)).await;

        // 获取输出
        let result = manager.get_output(&shell_id, None).await.unwrap();
        assert!(result.status == ShellStatus::Completed);
    }

    #[tokio::test]
    async fn test_kill_shell() {
        let manager = ShellManager::new();

        // 启动长时间运行的后台进程
        let shell_id = manager
            .execute_background("sleep 100", None)
            .await
            .unwrap();

        // 终止
        manager.kill(&shell_id).await.unwrap();

        // 等待状态更新
        tokio::time::sleep(Duration::from_millis(100)).await;

        let result = manager.get_output(&shell_id, None).await.unwrap();
        assert_eq!(result.status, ShellStatus::Killed);
    }
}

