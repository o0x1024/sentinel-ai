//! Docker sandbox for secure shell command execution

use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::process::Stdio;
use std::sync::Arc;
use tokio::process::Command;
use tokio::sync::RwLock;
use tokio::time::{timeout, Duration};
use tracing::{debug, info, warn};
use once_cell::sync::Lazy;

/// Docker sandbox configuration
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct DockerSandboxConfig {
    /// Docker image name
    pub image: String,
    /// Memory limit (e.g., "512m", "1g")
    pub memory_limit: String,
    /// CPU limit (e.g., "1.0" for 1 CPU)
    pub cpu_limit: String,
    /// Network mode ("none", "bridge", "host")
    pub network_mode: String,
    /// Enable read-only root filesystem
    pub read_only_rootfs: bool,
    /// Volume mounts (host_path -> container_path)
    pub volumes: HashMap<String, String>,
    /// Environment variables
    pub env_vars: HashMap<String, String>,
}

impl Default for DockerSandboxConfig {
    fn default() -> Self {
        Self {
            image: "sentinel-sandbox:latest".to_string(),
            memory_limit: "512m".to_string(),
            cpu_limit: "1.0".to_string(),
            network_mode: "bridge".to_string(),
            read_only_rootfs: false,
            volumes: HashMap::new(),
            env_vars: HashMap::new(),
        }
    }
}

/// Docker container pool entry
#[derive(Debug, Clone)]
struct ContainerInfo {
    created_at: std::time::Instant,
    last_used: std::time::Instant,
    use_count: usize,
}

/// Docker container pool
struct ContainerPool {
    containers: HashMap<String, ContainerInfo>,
    max_containers: usize,
    max_reuse_count: usize,
    max_idle_duration: Duration,
}

impl ContainerPool {
    fn new() -> Self {
        Self {
            containers: HashMap::new(),
            max_containers: 5,
            max_reuse_count: 10,
            max_idle_duration: Duration::from_secs(300), // 5 minutes
        }
    }

    /// Get or create a container
    async fn get_container(&mut self, config: &DockerSandboxConfig) -> Result<String, DockerError> {
        // Clean up stale containers
        self.cleanup_stale_containers().await;

        // Try to reuse existing container
        if let Some((id, _)) = self.find_reusable_container() {
            let id = id.clone();
            info!("Reusing container: {}", id);
            if let Some(info) = self.containers.get_mut(&id) {
                info.last_used = std::time::Instant::now();
                info.use_count += 1;
            }
            return Ok(id);
        }

        // Create new container if pool not full
        if self.containers.len() < self.max_containers {
            let container_id = Self::create_container(config).await?;
            info!("Created new container: {}", container_id);
            self.containers.insert(
                container_id.clone(),
                ContainerInfo {
                    created_at: std::time::Instant::now(),
                    last_used: std::time::Instant::now(),
                    use_count: 1,
                },
            );
            return Ok(container_id);
        }

        // Pool is full, remove oldest and create new
        if let Some(oldest_id) = self.find_oldest_container() {
            info!("Pool full, removing oldest container: {}", oldest_id);
            Self::remove_container(&oldest_id).await?;
            self.containers.remove(&oldest_id);
        }

        let container_id = Self::create_container(config).await?;
        self.containers.insert(
            container_id.clone(),
            ContainerInfo {
                created_at: std::time::Instant::now(),
                last_used: std::time::Instant::now(),
                use_count: 1,
            },
        );
        Ok(container_id)
    }

    /// Find a reusable container
    fn find_reusable_container(&self) -> Option<(&String, &ContainerInfo)> {
        self.containers
            .iter()
            .find(|(_, info)| info.use_count < self.max_reuse_count)
    }

    /// Find oldest container
    fn find_oldest_container(&self) -> Option<String> {
        self.containers
            .iter()
            .min_by_key(|(_, info)| info.created_at)
            .map(|(id, _)| id.clone())
    }

    /// Clean up stale containers
    async fn cleanup_stale_containers(&mut self) {
        let now = std::time::Instant::now();
        let stale_ids: Vec<String> = self
            .containers
            .iter()
            .filter(|(_, info)| {
                now.duration_since(info.last_used) > self.max_idle_duration
                    || info.use_count >= self.max_reuse_count
            })
            .map(|(id, _)| id.clone())
            .collect();

        for id in stale_ids {
            info!("Cleaning up stale container: {}", id);
            if let Err(e) = Self::remove_container(&id).await {
                warn!("Failed to remove stale container {}: {}", id, e);
            }
            self.containers.remove(&id);
        }
    }

    /// Create a new Docker container
    async fn create_container(config: &DockerSandboxConfig) -> Result<String, DockerError> {
        let mut args = vec![
            "run",
            "-d",
            "--rm",
            "--memory",
            &config.memory_limit,
            "--cpus",
            &config.cpu_limit,
            "--network",
            &config.network_mode,
        ];

        if config.read_only_rootfs {
            args.push("--read-only");
        }

        // Add volume mounts
        let volume_args: Vec<String> = config
            .volumes
            .iter()
            .flat_map(|(host, container)| vec!["-v".to_string(), format!("{}:{}", host, container)])
            .collect();
        let volume_refs: Vec<&str> = volume_args.iter().map(|s| s.as_str()).collect();
        args.extend(volume_refs);

        // Add environment variables
        let env_args: Vec<String> = config
            .env_vars
            .iter()
            .flat_map(|(k, v)| vec!["-e".to_string(), format!("{}={}", k, v)])
            .collect();
        let env_refs: Vec<&str> = env_args.iter().map(|s| s.as_str()).collect();
        args.extend(env_refs);

        args.push(&config.image);
        args.push("sleep");
        args.push("infinity");

        let output = Command::new("docker")
            .args(&args)
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .output()
            .await
            .map_err(|e| DockerError::CommandFailed(format!("Failed to create container: {}", e)))?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            return Err(DockerError::CommandFailed(format!(
                "Docker run failed: {}",
                stderr
            )));
        }

        let container_id = String::from_utf8_lossy(&output.stdout).trim().to_string();
        Ok(container_id)
    }

    /// Remove a container
    async fn remove_container(container_id: &str) -> Result<(), DockerError> {
        let output = Command::new("docker")
            .args(&["rm", "-f", container_id])
            .output()
            .await
            .map_err(|e| DockerError::CommandFailed(format!("Failed to remove container: {}", e)))?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            warn!("Failed to remove container {}: {}", container_id, stderr);
        }

        Ok(())
    }

    /// Cleanup all containers
    async fn cleanup_all(&mut self) {
        let ids: Vec<String> = self.containers.keys().cloned().collect();
        for id in ids {
            if let Err(e) = Self::remove_container(&id).await {
                warn!("Failed to cleanup container {}: {}", id, e);
            }
        }
        self.containers.clear();
    }
}

/// Global container pool
static CONTAINER_POOL: Lazy<Arc<RwLock<ContainerPool>>> =
    Lazy::new(|| Arc::new(RwLock::new(ContainerPool::new())));

/// Docker sandbox errors
#[derive(Debug, thiserror::Error)]
pub enum DockerError {
    #[error("Docker command failed: {0}")]
    CommandFailed(String),
    #[error("Docker not available: {0}")]
    NotAvailable(String),
    #[error("Image not found: {0}")]
    ImageNotFound(String),
    #[error("Container execution failed: {0}")]
    ExecutionFailed(String),
}

/// Docker sandbox executor
pub struct DockerSandbox {
    config: DockerSandboxConfig,
}

impl DockerSandbox {
    pub fn new(config: DockerSandboxConfig) -> Self {
        Self { config }
    }

    /// Check if Docker is available
    pub async fn is_docker_available() -> bool {
        let output = Command::new("docker")
            .arg("version")
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .status()
            .await;

        matches!(output, Ok(status) if status.success())
    }

    /// Build sandbox image
    pub async fn build_image(dockerfile_path: &str, image_name: &str) -> Result<(), DockerError> {
        info!("Building Docker image: {}", image_name);

        let output = Command::new("docker")
            .args(&["build", "-t", image_name, "-f", dockerfile_path, "."])
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .output()
            .await
            .map_err(|e| DockerError::CommandFailed(format!("Failed to build image: {}", e)))?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            return Err(DockerError::CommandFailed(format!(
                "Docker build failed: {}",
                stderr
            )));
        }

        info!("Successfully built image: {}", image_name);
        Ok(())
    }

    /// Check if image exists
    pub async fn image_exists(image_name: &str) -> bool {
        let output = Command::new("docker")
            .args(&["images", "-q", image_name])
            .stdout(Stdio::piped())
            .output()
            .await;

        if let Ok(output) = output {
            !output.stdout.is_empty()
        } else {
            false
        }
    }

    /// Execute command in Docker sandbox
    pub async fn execute(
        &self,
        command: &str,
        timeout_secs: u64,
    ) -> Result<(String, String, i32), DockerError> {
        // Get or create container
        let container_id = {
            let mut pool = CONTAINER_POOL.write().await;
            pool.get_container(&self.config).await?
        };

        debug!("Executing command in container {}: {}", container_id, command);

        // Execute command in container
        let timeout_duration = Duration::from_secs(timeout_secs);
        let result = timeout(
            timeout_duration,
            Command::new("docker")
                .args(&["exec", &container_id, "bash", "-c", command])
                .stdout(Stdio::piped())
                .stderr(Stdio::piped())
                .output(),
        )
        .await;

        match result {
            Ok(Ok(output)) => {
                let stdout = String::from_utf8_lossy(&output.stdout).to_string();
                let stderr = String::from_utf8_lossy(&output.stderr).to_string();
                let exit_code = output.status.code().unwrap_or(-1);
                Ok((stdout, stderr, exit_code))
            }
            Ok(Err(e)) => Err(DockerError::ExecutionFailed(format!(
                "Failed to execute command: {}",
                e
            ))),
            Err(_) => Err(DockerError::ExecutionFailed(format!(
                "Command timeout after {} seconds",
                timeout_secs
            ))),
        }
    }

    /// Cleanup all containers
    pub async fn cleanup_all() {
        let mut pool = CONTAINER_POOL.write().await;
        pool.cleanup_all().await;
    }
}

/// Initialize Docker sandbox (build image if needed)
pub async fn init_docker_sandbox(
    dockerfile_path: Option<&str>,
    image_name: Option<&str>,
) -> Result<(), DockerError> {
    if !DockerSandbox::is_docker_available().await {
        return Err(DockerError::NotAvailable(
            "Docker is not available on this system".to_string(),
        ));
    }

    let image = image_name.unwrap_or("sentinel-sandbox:latest");

    // Check if image exists
    if !DockerSandbox::image_exists(image).await {
        if let Some(dockerfile) = dockerfile_path {
            info!("Image {} not found, building from {}", image, dockerfile);
            DockerSandbox::build_image(dockerfile, image).await?;
        } else {
            return Err(DockerError::ImageNotFound(format!(
                "Image {} not found and no Dockerfile provided",
                image
            )));
        }
    }

    info!("Docker sandbox initialized with image: {}", image);
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_docker_available() {
        let available = DockerSandbox::is_docker_available().await;
        println!("Docker available: {}", available);
    }

    #[tokio::test]
    async fn test_execute_command() {
        if !DockerSandbox::is_docker_available().await {
            println!("Docker not available, skipping test");
            return;
        }

        let config = DockerSandboxConfig::default();
        let sandbox = DockerSandbox::new(config);

        match sandbox.execute("echo 'Hello from Docker'", 10).await {
            Ok((stdout, stderr, code)) => {
                println!("stdout: {}", stdout);
                println!("stderr: {}", stderr);
                println!("exit code: {}", code);
            }
            Err(e) => {
                println!("Error: {}", e);
            }
        }

        DockerSandbox::cleanup_all().await;
    }
}
