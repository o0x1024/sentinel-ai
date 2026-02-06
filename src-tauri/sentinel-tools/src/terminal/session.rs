//! Terminal session management

use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tokio::process::Command;
use tokio::sync::{mpsc, RwLock};
use tracing::{error, info, warn};
use portable_pty::{native_pty_system, CommandBuilder, MasterPty, PtySize};

/// Execution mode for terminal session
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum ExecutionMode {
    /// Execute in Docker container
    Docker,
    /// Execute on host machine
    Host,
}

/// Terminal session configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TerminalSessionConfig {
    /// Execution mode (Docker or Host)
    pub execution_mode: ExecutionMode,
    /// Docker image name (only used when execution_mode is Docker)
    pub docker_image: String,
    /// Working directory
    pub working_dir: Option<String>,
    /// Environment variables
    pub env_vars: std::collections::HashMap<String, String>,
    /// Shell to use (bash, sh, zsh, etc.)
    pub shell: String,
    /// Optional command to execute immediately after session starts
    pub initial_command: Option<String>,
    /// Reuse existing container if available (only for Docker mode)
    pub reuse_container: bool,
    /// Container name for reuse identification (only for Docker mode)
    pub container_name: Option<String>,
}

impl Default for TerminalSessionConfig {
    fn default() -> Self {
        let mut env_vars = std::collections::HashMap::new();
        env_vars.insert("TERM".to_string(), "xterm-256color".to_string());

        Self {
            execution_mode: ExecutionMode::Docker,
            docker_image: "sentinel-sandbox:latest".to_string(),
            working_dir: Some("/workspace".to_string()),
            env_vars,
            shell: "bash".to_string(),
            initial_command: None,
            reuse_container: true,
            container_name: Some("sentinel-sandbox-main".to_string()),
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
    pty_master: Option<Arc<std::sync::Mutex<Box<dyn MasterPty + Send>>>>,
    stdin_tx: Option<mpsc::UnboundedSender<Vec<u8>>>,
    output_txs: Arc<RwLock<Vec<mpsc::UnboundedSender<Vec<u8>>>>>,
    output_history: Arc<RwLock<Vec<Vec<u8>>>>,
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
            pty_master: None,
            stdin_tx: None,
            output_txs: Arc::new(RwLock::new(Vec::new())),
            output_history: Arc::new(RwLock::new(Vec::new())),
            last_activity: Arc::new(RwLock::new(std::time::Instant::now())),
        })
    }

    /// Add an output subscriber (with history replay for UI)
    pub async fn add_subscriber(&self, tx: mpsc::UnboundedSender<Vec<u8>>) {
        // Send history to new subscriber
        let history = self.output_history.read().await;
        info!("[Terminal Session {}] Adding subscriber with history, chunks: {}", self.id, history.len());
        
        for (i, data) in history.iter().enumerate() {
            info!("[Terminal Session {}] Sending history chunk {}: {} bytes", self.id, i, data.len());
            if let Err(e) = tx.send(data.clone()) {
                error!("[Terminal Session {}] Failed to send history chunk {}: {}", self.id, i, e);
            }
        }
        
        self.output_txs.write().await.push(tx);
        info!("[Terminal Session {}] Subscriber added, total subscribers: {}", 
            self.id, self.output_txs.read().await.len());
    }

    /// Add an output subscriber without history (for LLM - only captures new output)
    pub async fn add_subscriber_no_history(&self, tx: mpsc::UnboundedSender<Vec<u8>>) {
        info!("[Terminal Session {}] Adding subscriber without history (LLM mode)", self.id);
        self.output_txs.write().await.push(tx);
        info!("[Terminal Session {}] Subscriber added, total subscribers: {}", 
            self.id, self.output_txs.read().await.len());
    }

    /// Broadcast output to all subscribers
    async fn broadcast_output(
        output_txs: Arc<RwLock<Vec<mpsc::UnboundedSender<Vec<u8>>>>>, 
        output_history: Arc<RwLock<Vec<Vec<u8>>>>,
        data: Vec<u8>
    ) {
        // Add to history (keep last 1000 chunks)
        {
            let mut history = output_history.write().await;
            history.push(data.clone());
            if history.len() > 1000 {
                history.remove(0);
            }
        }

        let mut txs = output_txs.write().await;
        txs.retain(|tx| {
            tx.send(data.clone()).is_ok()
        });
    }

    /// Start the terminal session
    pub async fn start(
        &mut self,
        output_tx: mpsc::UnboundedSender<Vec<u8>>,
    ) -> Result<(), String> {
        info!("Starting terminal session: {}", self.id);
        self.add_subscriber(output_tx).await;

        match self.config.execution_mode {
            ExecutionMode::Docker => self.start_docker_session().await,
            ExecutionMode::Host => self.start_host_session().await,
        }
    }

    /// Start Docker-based session with PTY support
    async fn start_docker_session(
        &mut self,
    ) -> Result<(), String> {
        // Get or create container
        let container_id = self.get_or_create_container().await?;
        self.container_id = Some(container_id.clone());

        // Create PTY pair
        let pty_system = native_pty_system();
        let pty_pair = pty_system
            .openpty(PtySize {
                rows: 30,
                cols: 120,
                pixel_width: 0,
                pixel_height: 0,
            })
            .map_err(|e| format!("Failed to create PTY: {}", e))?;

        // Build docker exec command with PTY support
        let mut cmd_builder = CommandBuilder::new("docker");
        cmd_builder.arg("exec");
        cmd_builder.arg("-it");  // ✅ Now we can use -it with PTY!

        for (key, value) in &self.config.env_vars {
            cmd_builder.arg("-e");
            cmd_builder.arg(format!("{}={}", key, value));
        }
        
        // For Kali images, use sandbox user
        if self.config.docker_image.contains("kali") {
            cmd_builder.arg("-u");
            cmd_builder.arg("sandbox");
        }
        
        // Add working directory
        if let Some(ref wd) = self.config.working_dir {
            cmd_builder.arg("-w");
            cmd_builder.arg(wd);
        }
        
        // Add container_id and shell
        cmd_builder.arg(&container_id);
        cmd_builder.arg(&self.config.shell);

        // Spawn command through PTY
        let _child = pty_pair.slave.spawn_command(cmd_builder)
            .map_err(|e| format!("Failed to spawn docker exec with PTY: {}", e))?;

        // Get PTY master for reading/writing
        let mut pty_reader = pty_pair.master.try_clone_reader()
            .map_err(|e| format!("Failed to clone PTY reader: {}", e))?;
        let mut pty_writer = pty_pair.master.take_writer()
            .map_err(|e| format!("Failed to get PTY writer: {}", e))?;
        self.pty_master = Some(Arc::new(std::sync::Mutex::new(pty_pair.master)));

        // Setup stdin channel for writing to PTY
        let (stdin_tx, mut stdin_rx) = mpsc::unbounded_channel::<Vec<u8>>();
        self.stdin_tx = Some(stdin_tx.clone());

        // Stdin writer task (write to PTY master)
        tokio::task::spawn_blocking(move || {
            use std::io::Write;
            while let Some(data) = stdin_rx.blocking_recv() {
                if let Err(e) = pty_writer.write_all(&data) {
                    error!("Failed to write to PTY: {}", e);
                    break;
                }
                if let Err(e) = pty_writer.flush() {
                    error!("Failed to flush PTY: {}", e);
                    break;
                }
            }
        });

        // PTY reader task (read from PTY master)
        let output_txs_clone = self.output_txs.clone();
        let output_history_clone = self.output_history.clone();
        let last_activity = self.last_activity.clone();
        
        tokio::task::spawn_blocking(move || {
            use std::io::Read;
            let mut buffer = [0u8; 8192];
            loop {
                match pty_reader.read(&mut buffer) {
                    Ok(0) => {
                        info!("PTY reader reached EOF");
                        break;
                    }
                    Ok(n) => {
                        let data = buffer[..n].to_vec();
                        // Broadcast must be done in async context
                        tokio::runtime::Handle::current().block_on(async {
                            *last_activity.write().await = std::time::Instant::now();
                            Self::broadcast_output(
                                output_txs_clone.clone(),
                                output_history_clone.clone(),
                                data,
                            )
                            .await;
                        });
                    }
                    Err(e) => {
                        error!("Failed to read from PTY: {}", e);
                        break;
                    }
                }
            }
            info!("PTY reader task ended");
        });

        // Note: We don't store the child process for PTY mode
        // The PTY system manages the process lifecycle
        *self.state.write().await = SessionState::Running;

        info!("Terminal session started with PTY: {}", self.id);
        
        // Execute initial_command if provided
        if let Some(ref initial_cmd) = self.config.initial_command {
            if !initial_cmd.is_empty() {
                info!("Executing initial command: {}", initial_cmd);
                // Wait a bit for shell to be ready
                tokio::time::sleep(tokio::time::Duration::from_millis(500)).await;
                
                // Send the command with newline
                let cmd_with_newline = format!("{}\n", initial_cmd);
                if let Err(e) = stdin_tx.send(cmd_with_newline.into_bytes()) {
                    error!("Failed to send initial command: {}", e);
                }
            }
        }
        
        Ok(())
    }

    /// Start host-based session with PTY support
    async fn start_host_session(
        &mut self,
    ) -> Result<(), String> {
        warn!("Starting terminal on host (less secure)");

        let pty_system = native_pty_system();
        let pty_pair = pty_system
            .openpty(PtySize {
                rows: 30,
                cols: 120,
                pixel_width: 0,
                pixel_height: 0,
            })
            .map_err(|e| format!("Failed to create host PTY: {}", e))?;

        let mut cmd_builder = CommandBuilder::new(&self.config.shell);
        for (key, value) in &self.config.env_vars {
            cmd_builder.env(key, value);
        }

        let _child = pty_pair
            .slave
            .spawn_command(cmd_builder)
            .map_err(|e| format!("Failed to start host shell with PTY: {}", e))?;

        let mut pty_reader = pty_pair
            .master
            .try_clone_reader()
            .map_err(|e| format!("Failed to clone host PTY reader: {}", e))?;
        let mut pty_writer = pty_pair
            .master
            .take_writer()
            .map_err(|e| format!("Failed to get host PTY writer: {}", e))?;
        self.pty_master = Some(Arc::new(std::sync::Mutex::new(pty_pair.master)));

        let (stdin_tx, mut stdin_rx) = mpsc::unbounded_channel::<Vec<u8>>();
        self.stdin_tx = Some(stdin_tx.clone());

        tokio::task::spawn_blocking(move || {
            use std::io::Write;
            while let Some(data) = stdin_rx.blocking_recv() {
                if let Err(e) = pty_writer.write_all(&data) {
                    error!("Failed to write to host PTY: {}", e);
                    break;
                }
                if let Err(e) = pty_writer.flush() {
                    error!("Failed to flush host PTY: {}", e);
                    break;
                }
            }
        });

        let output_txs_clone = self.output_txs.clone();
        let output_history_clone = self.output_history.clone();
        let last_activity = self.last_activity.clone();
        tokio::task::spawn_blocking(move || {
            use std::io::Read;
            let mut buffer = [0u8; 8192];
            loop {
                match pty_reader.read(&mut buffer) {
                    Ok(0) => {
                        info!("Host PTY reader reached EOF");
                        break;
                    }
                    Ok(n) => {
                        let data = buffer[..n].to_vec();
                        tokio::runtime::Handle::current().block_on(async {
                            *last_activity.write().await = std::time::Instant::now();
                            Self::broadcast_output(
                                output_txs_clone.clone(),
                                output_history_clone.clone(),
                                data,
                            )
                            .await;
                        });
                    }
                    Err(e) => {
                        error!("Failed to read from host PTY: {}", e);
                        break;
                    }
                }
            }
        });

        *self.state.write().await = SessionState::Running;

        if let Some(ref wd) = self.config.working_dir {
            let cd_command = format!("cd {}\n", wd);
            let _ = stdin_tx.send(cd_command.into_bytes());
        }

        if let Some(ref initial_cmd) = self.config.initial_command {
            if !initial_cmd.is_empty() {
                let cmd_with_newline = format!("{}\n", initial_cmd);
                let _ = stdin_tx.send(cmd_with_newline.into_bytes());
            }
        }

        Ok(())
    }

    /// Get or create Docker container (reuse if available)
    async fn get_or_create_container(&self) -> Result<String, String> {
        // Try to reuse existing container if configured
        if self.config.reuse_container {
            if let Some(container_id) = self.find_reusable_container().await? {
                info!("Reusing existing container: {}", container_id);
                
                // Make sure container is running
                self.ensure_container_running(&container_id).await?;
                
                return Ok(container_id);
            }
        }
        
        // No reusable container found, create new one
        self.create_container().await
    }

    /// Find reusable container
    async fn find_reusable_container(&self) -> Result<Option<String>, String> {
        let container_name = match &self.config.container_name {
            Some(name) => name,
            None => return Ok(None),
        };

        info!("Looking for reusable container: {}", container_name);

        // List containers with the specified name
        let output = Command::new("docker")
            .args(&[
                "ps",
                "-a",
                "--filter",
                &format!("name=^{}$", container_name),
                "--format",
                "{{.ID}}",
            ])
            .output()
            .await
            .map_err(|e| format!("Failed to list containers: {}", e))?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            return Err(format!("Docker ps failed: {}", stderr));
        }

        let container_id = String::from_utf8_lossy(&output.stdout).trim().to_string();
        
        if container_id.is_empty() {
            info!("No reusable container found");
            Ok(None)
        } else {
            info!("Found reusable container: {}", container_id);
            Ok(Some(container_id))
        }
    }

    /// Ensure container is running
    async fn ensure_container_running(&self, container_id: &str) -> Result<(), String> {
        // Check container state
        let output = Command::new("docker")
            .args(&["inspect", "-f", "{{.State.Running}}", container_id])
            .output()
            .await
            .map_err(|e| format!("Failed to inspect container: {}", e))?;

        let is_running = String::from_utf8_lossy(&output.stdout).trim() == "true";

        if !is_running {
            info!("Container {} is not running, starting it", container_id);
            let output = Command::new("docker")
                .args(&["start", container_id])
                .output()
                .await
                .map_err(|e| format!("Failed to start container: {}", e))?;

            if !output.status.success() {
                let stderr = String::from_utf8_lossy(&output.stderr);
                return Err(format!("Failed to start container: {}", stderr));
            }

            info!("Container started successfully");
        } else {
            info!("Container is already running");
        }

        Ok(())
    }

    /// Create Docker container
    async fn create_container(&self) -> Result<String, String> {
        info!("Creating Docker container with image: {}", self.config.docker_image);

        // Check if we should use a non-root user
        let use_sandbox_user = self.config.docker_image.contains("kali");
        
        let mut args = vec![
            "run",
            "-d",
            "-i",
        ];

        // Add container name if specified (for reuse)
        if let Some(ref name) = self.config.container_name {
            args.push("--name");
            args.push(name);
        } else {
            // If not reusing, use --rm
            args.push("--rm");
        }

        // Relax permissions for tools like nmap that need raw sockets / elevated exec behavior.
        args.extend_from_slice(&[
            "--cap-add=NET_RAW",
            "--cap-add=NET_ADMIN",
            "--security-opt",
            "no-new-privileges=false",
        ]);
        
        // Add working directory mount if specified
        if let Some(ref wd) = self.config.working_dir {
            args.push("-v");
            args.push("/tmp/workspace:/workspace");
            args.push("-w");
            args.push(wd);
        }
        
        args.push(&self.config.docker_image);
        args.push("sleep");
        args.push("infinity");

        let output = Command::new("docker")
            .args(&args)
            .output()
            .await
            .map_err(|e| format!("Failed to create container: {}", e))?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            
            // If container name already exists, try to remove it and retry
            if stderr.contains("already in use") {
                warn!("Container name already in use, removing old container");
                if let Some(ref name) = self.config.container_name {
                    let _ = Command::new("docker")
                        .args(&["rm", "-f", name])
                        .output()
                        .await;
                    
                    // Retry creation
                    let retry_output = Command::new("docker")
                        .args(&args)
                        .output()
                        .await
                        .map_err(|e| format!("Failed to create container (retry): {}", e))?;
                    
                    if !retry_output.status.success() {
                        let retry_stderr = String::from_utf8_lossy(&retry_output.stderr);
                        return Err(format!("Docker run failed (retry): {}", retry_stderr));
                    }
                    
                    let container_id = String::from_utf8_lossy(&retry_output.stdout).trim().to_string();
                    info!("Container created (after retry): {}", container_id);
                    return self.setup_container(&container_id, use_sandbox_user).await;
                }
            }
            
            return Err(format!("Docker run failed: {}", stderr));
        }

        let container_id = String::from_utf8_lossy(&output.stdout).trim().to_string();
        info!("Container created: {}", container_id);
        
        self.setup_container(&container_id, use_sandbox_user).await
    }

    /// Setup container (create user, directories, etc.)
    async fn setup_container(&self, container_id: &str, use_sandbox_user: bool) -> Result<String, String> {
        // For Kali images, create sandbox user if it doesn't exist
        if use_sandbox_user {
            info!("Setting up sandbox user in container");
            let setup_commands = vec![
                // Create sandbox user if not exists
                "id -u sandbox &>/dev/null || useradd -m -s /bin/bash sandbox",
                // Create workspace directory
                "mkdir -p /workspace",
                // Set ownership
                "chown -R sandbox:sandbox /workspace 2>/dev/null || true",
            ];
            
            for cmd in setup_commands {
                let result = Command::new("docker")
                    .args(&["exec", "--user", "root", container_id, "bash", "-c", cmd])
                    .output()
                    .await;
                
                if let Err(e) = result {
                    warn!("Failed to execute setup command '{}': {}", cmd, e);
                }
            }
            
            info!("Sandbox user setup completed");
        }
        
        Ok(container_id.to_string())
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

    /// Resize PTY window (no-op for non-PTY sessions)
    pub async fn resize(&self, rows: u16, cols: u16) -> Result<(), String> {
        let Some(pty_master) = &self.pty_master else {
            return Ok(());
        };

        let guard = pty_master
            .lock()
            .map_err(|_| "Failed to acquire PTY lock".to_string())?;
        guard
            .resize(PtySize {
                rows,
                cols,
                pixel_width: 0,
                pixel_height: 0,
            })
            .map_err(|e| format!("Failed to resize PTY: {}", e))
    }

    /// Check if the session is healthy (stdin is open)
    pub fn is_healthy(&self) -> bool {
        if let Some(ref tx) = self.stdin_tx {
            !tx.is_closed()
        } else {
            false
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

    /// Update last activity timestamp
    pub async fn touch(&self) {
        *self.last_activity.write().await = std::time::Instant::now();
    }

    /// Get container ID
    pub fn container_id(&self) -> Option<String> {
        self.container_id.clone()
    }

    /// Check if container is healthy
    pub async fn is_container_healthy(&self) -> bool {
        if let Some(ref container_id) = self.container_id {
            let output = Command::new("docker")
                .args(&["inspect", "-f", "{{.State.Running}}", container_id])
                .output()
                .await;

            if let Ok(output) = output {
                if output.status.success() {
                    let is_running = String::from_utf8_lossy(&output.stdout).trim() == "true";
                    return is_running;
                }
            }
        }
        false
    }

    /// Stop the terminal session
    pub async fn stop(&mut self) -> Result<(), String> {
        info!("Stopping terminal session: {}", self.id);

        *self.state.write().await = SessionState::Stopped;

        self.stdin_tx = None;
        self.pty_master = None;

        // Only remove container if not configured for reuse
        if !self.config.reuse_container {
            if let Some(ref container_id) = self.container_id {
                info!("Removing container (not configured for reuse): {}", container_id);
                let _ = Command::new("docker")
                    .args(&["rm", "-f", container_id])
                    .output()
                    .await;
            }
        } else {
            info!("Keeping container for reuse: {:?}", self.container_id);
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
