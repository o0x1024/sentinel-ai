//! Terminal session management

use serde::{Deserialize, Serialize};
use std::process::Stdio;
use std::sync::Arc;
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use tokio::process::{Child, Command};
use tokio::sync::{mpsc, RwLock};
use tracing::{debug, error, info, warn};

/// Terminal session configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TerminalSessionConfig {
    /// Use Docker container
    pub use_docker: bool,
    /// Docker image name
    pub docker_image: String,
    /// Working directory
    pub working_dir: Option<String>,
    /// Environment variables
    pub env_vars: std::collections::HashMap<String, String>,
    /// Shell to use (bash, sh, zsh, etc.)
    pub shell: String,
}

impl Default for TerminalSessionConfig {
    fn default() -> Self {
        Self {
            use_docker: true,
            docker_image: "sentinel-sandbox:latest".to_string(),
            working_dir: Some("/workspace".to_string()),
            env_vars: std::collections::HashMap::new(),
            shell: "bash".to_string(),
        }
    }
}

/// Terminal session state
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
pub enum SessionState {
    Starting,
    Running,
    Stopped,
    Error,
}

/// Terminal session
pub struct TerminalSession {
    pub id: String,
    pub config: TerminalSessionConfig,
    state: Arc<RwLock<SessionState>>,
    container_id: Option<String>,
    process: Option<Child>,
    stdin_tx: Option<mpsc::UnboundedSender<Vec<u8>>>,
    last_activity: Arc<RwLock<std::time::Instant>>,
}

impl TerminalSession {
    /// Create a new terminal session
    pub async fn new(id: String, config: TerminalSessionConfig) -> Result<Self, String> {
        Ok(Self {
            id,
            config,
            state: Arc::new(RwLock::new(SessionState::Starting)),
            container_id: None,
            process: None,
            stdin_tx: None,
            last_activity: Arc::new(RwLock::new(std::time::Instant::now())),
        })
    }

    /// Start the terminal session
    pub async fn start(
        &mut self,
        output_tx: mpsc::UnboundedSender<Vec<u8>>,
    ) -> Result<(), String> {
        info!("Starting terminal session: {}", self.id);

        if self.config.use_docker {
            self.start_docker_session(output_tx).await
        } else {
            self.start_host_session(output_tx).await
        }
    }

    /// Start Docker-based session
    async fn start_docker_session(
        &mut self,
        output_tx: mpsc::UnboundedSender<Vec<u8>>,
    ) -> Result<(), String> {
        // Create container
        let container_id = self.create_container().await?;
        self.container_id = Some(container_id.clone());

        // Start interactive shell in container
        let mut cmd = Command::new("docker");
        cmd.args(&["exec", "-i", &container_id, &self.config.shell]);

        if let Some(ref wd) = self.config.working_dir {
            cmd.args(&["-w", wd]);
        }

        cmd.stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped());

        let mut child = cmd
            .spawn()
            .map_err(|e| format!("Failed to start shell: {}", e))?;

        // Setup stdin channel
        let stdin = child.stdin.take().ok_or("Failed to get stdin")?;
        let (stdin_tx, mut stdin_rx) = mpsc::unbounded_channel::<Vec<u8>>();
        self.stdin_tx = Some(stdin_tx);

        // Stdin writer task
        tokio::spawn(async move {
            let mut stdin = stdin;
            while let Some(data) = stdin_rx.recv().await {
                if let Err(e) = stdin.write_all(&data).await {
                    error!("Failed to write to stdin: {}", e);
                    break;
                }
                if let Err(e) = stdin.flush().await {
                    error!("Failed to flush stdin: {}", e);
                    break;
                }
            }
        });

        // Setup stdout/stderr readers
        let stdout = child.stdout.take().ok_or("Failed to get stdout")?;
        let stderr = child.stderr.take().ok_or("Failed to get stderr")?;

        let output_tx_clone = output_tx.clone();
        let last_activity = self.last_activity.clone();

        // Stdout reader task
        tokio::spawn(async move {
            let mut reader = BufReader::new(stdout);
            let mut buffer = Vec::new();
            loop {
                buffer.clear();
                match reader.read_until(b'\n', &mut buffer).await {
                    Ok(0) => break, // EOF
                    Ok(_) => {
                        *last_activity.write().await = std::time::Instant::now();
                        if output_tx_clone.send(buffer.clone()).is_err() {
                            break;
                        }
                    }
                    Err(e) => {
                        error!("Failed to read stdout: {}", e);
                        break;
                    }
                }
            }
        });

        // Stderr reader task
        let last_activity = self.last_activity.clone();
        tokio::spawn(async move {
            let mut reader = BufReader::new(stderr);
            let mut buffer = Vec::new();
            loop {
                buffer.clear();
                match reader.read_until(b'\n', &mut buffer).await {
                    Ok(0) => break,
                    Ok(_) => {
                        *last_activity.write().await = std::time::Instant::now();
                        if output_tx.send(buffer.clone()).is_err() {
                            break;
                        }
                    }
                    Err(e) => {
                        error!("Failed to read stderr: {}", e);
                        break;
                    }
                }
            }
        });

        self.process = Some(child);
        *self.state.write().await = SessionState::Running;

        info!("Terminal session started: {}", self.id);
        Ok(())
    }

    /// Start host-based session
    async fn start_host_session(
        &mut self,
        output_tx: mpsc::UnboundedSender<Vec<u8>>,
    ) -> Result<(), String> {
        warn!("Starting terminal on host (less secure)");

        let mut cmd = Command::new(&self.config.shell);
        cmd.stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped());

        if let Some(ref wd) = self.config.working_dir {
            cmd.current_dir(wd);
        }

        for (key, value) in &self.config.env_vars {
            cmd.env(key, value);
        }

        let mut child = cmd
            .spawn()
            .map_err(|e| format!("Failed to start shell: {}", e))?;

        // Similar setup as Docker session
        let stdin = child.stdin.take().ok_or("Failed to get stdin")?;
        let (stdin_tx, mut stdin_rx) = mpsc::unbounded_channel::<Vec<u8>>();
        self.stdin_tx = Some(stdin_tx);

        tokio::spawn(async move {
            let mut stdin = stdin;
            while let Some(data) = stdin_rx.recv().await {
                if stdin.write_all(&data).await.is_err() {
                    break;
                }
                let _ = stdin.flush().await;
            }
        });

        let stdout = child.stdout.take().ok_or("Failed to get stdout")?;
        let stderr = child.stderr.take().ok_or("Failed to get stderr")?;

        let output_tx_clone = output_tx.clone();
        let last_activity = self.last_activity.clone();

        tokio::spawn(async move {
            let mut reader = BufReader::new(stdout);
            let mut buffer = Vec::new();
            loop {
                buffer.clear();
                match reader.read_until(b'\n', &mut buffer).await {
                    Ok(0) => break,
                    Ok(_) => {
                        *last_activity.write().await = std::time::Instant::now();
                        if output_tx_clone.send(buffer.clone()).is_err() {
                            break;
                        }
                    }
                    Err(_) => break,
                }
            }
        });

        let last_activity = self.last_activity.clone();
        tokio::spawn(async move {
            let mut reader = BufReader::new(stderr);
            let mut buffer = Vec::new();
            loop {
                buffer.clear();
                match reader.read_until(b'\n', &mut buffer).await {
                    Ok(0) => break,
                    Ok(_) => {
                        *last_activity.write().await = std::time::Instant::now();
                        if output_tx.send(buffer.clone()).is_err() {
                            break;
                        }
                    }
                    Err(_) => break,
                }
            }
        });

        self.process = Some(child);
        *self.state.write().await = SessionState::Running;

        Ok(())
    }

    /// Create Docker container
    async fn create_container(&self) -> Result<String, String> {
        debug!("Creating Docker container");

        let output = Command::new("docker")
            .args(&[
                "run",
                "-d",
                "--rm",
                "-i",
                &self.config.docker_image,
                "sleep",
                "infinity",
            ])
            .output()
            .await
            .map_err(|e| format!("Failed to create container: {}", e))?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            return Err(format!("Docker run failed: {}", stderr));
        }

        let container_id = String::from_utf8_lossy(&output.stdout).trim().to_string();
        debug!("Container created: {}", container_id);
        Ok(container_id)
    }

    /// Write data to terminal
    pub async fn write(&self, data: Vec<u8>) -> Result<(), String> {
        *self.last_activity.write().await = std::time::Instant::now();

        if let Some(ref tx) = self.stdin_tx {
            tx.send(data)
                .map_err(|_| "Failed to send data to terminal".to_string())?;
            Ok(())
        } else {
            Err("Terminal not started".to_string())
        }
    }

    /// Get session state
    pub async fn state(&self) -> SessionState {
        *self.state.read().await
    }

    /// Get last activity time
    pub async fn last_activity(&self) -> std::time::Instant {
        *self.last_activity.read().await
    }

    /// Stop the terminal session
    pub async fn stop(&mut self) -> Result<(), String> {
        info!("Stopping terminal session: {}", self.id);

        *self.state.write().await = SessionState::Stopped;

        // Kill process
        if let Some(mut process) = self.process.take() {
            let _ = process.kill().await;
        }

        // Remove container
        if let Some(ref container_id) = self.container_id {
            let _ = Command::new("docker")
                .args(&["rm", "-f", container_id])
                .output()
                .await;
        }

        Ok(())
    }
}

impl Drop for TerminalSession {
    fn drop(&mut self) {
        // Cleanup will be handled by stop()
        // debug!("Dropping terminal session: {}", self.id);
    }
}
