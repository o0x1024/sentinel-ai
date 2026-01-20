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
    /// Reuse container across restarts
    pub reuse_container: bool,
    /// Container name for persistent reuse
    pub container_name: Option<String>,
}

impl Default for DockerSandboxConfig {
    fn default() -> Self {
        Self {
            image: "sentinel-sandbox:latest".to_string(),
            memory_limit: "2g".to_string(),
            cpu_limit: "4.0".to_string(),
            network_mode: "bridge".to_string(),
            read_only_rootfs: false,
            volumes: HashMap::new(),
            env_vars: HashMap::new(),
            reuse_container: true,
            container_name: Some("sentinel-sandbox-main".to_string()),
        }
    }
}

/// Docker container pool entry
#[derive(Debug, Clone)]
struct ContainerInfo {
    last_used: std::time::Instant,
    use_count: usize,
}

/// Docker container pool
struct ContainerPool {
    containers: HashMap<String, ContainerInfo>,
    max_reuse_count: usize,
    max_idle_duration: Duration,
}

impl ContainerPool {
    fn new() -> Self {
        Self {
            containers: HashMap::new(),
            max_reuse_count: 10,
            max_idle_duration: Duration::from_secs(300), // 5 minutes
        }
    }

    /// Get or create a container
    async fn get_container(&mut self, config: &DockerSandboxConfig) -> Result<String, DockerError> {
        // If reuse is enabled and container name is specified, try persistent reuse
        if config.reuse_container {
            if let Some(container_id) = Self::find_persistent_container(config).await? {
                info!("Reusing persistent container: {}", container_id);
                Self::ensure_container_running(&container_id).await?;
                
                // Update tracking info
                if let Some(info) = self.containers.get_mut(&container_id) {
                    info.last_used = std::time::Instant::now();
                    info.use_count += 1;
                } else {
                    // Add to tracking if not already tracked
                    self.containers.insert(
                        container_id.clone(),
                        ContainerInfo {
                            last_used: std::time::Instant::now(),
                            use_count: 1,
                        },
                    );
                }
                
                return Ok(container_id);
            }
        }

        // Clean up stale containers (only for pool-based containers)
        if !config.reuse_container {
            self.cleanup_stale_containers().await;
        }

        // Try to reuse existing container from pool
        if !config.reuse_container {
            if let Some((id, _)) = self.find_reusable_container() {
                let id = id.clone();
                info!("Reusing pooled container: {}", id);
                if let Some(info) = self.containers.get_mut(&id) {
                    info.last_used = std::time::Instant::now();
                    info.use_count += 1;
                }
                return Ok(id);
            }
        }

        // Create new container
        let container_id = Self::create_container(config).await?;
        info!("Created new container: {}", container_id);
        self.containers.insert(
            container_id.clone(),
            ContainerInfo {
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


    /// Find persistent container by name
    async fn find_persistent_container(config: &DockerSandboxConfig) -> Result<Option<String>, DockerError> {
        let container_name = match &config.container_name {
            Some(name) => name,
            None => return Ok(None),
        };

        info!("Looking for persistent container: {}", container_name);

        let output = Command::new("docker")
            .args(&[
                "ps",
                "-a",
                "--filter",
                &format!("name=^{}$", container_name),
                "--format",
                "{{.ID}}",
            ])
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .output()
            .await
            .map_err(|e| DockerError::CommandFailed(format!("Failed to list containers: {}", e)))?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            return Err(DockerError::CommandFailed(format!("Docker ps failed: {}", stderr)));
        }

        let container_id = String::from_utf8_lossy(&output.stdout).trim().to_string();
        
        if container_id.is_empty() {
            info!("No persistent container found");
            Ok(None)
        } else {
            info!("Found persistent container: {}", container_id);
            Ok(Some(container_id))
        }
    }

    /// Ensure container is running
    async fn ensure_container_running(container_id: &str) -> Result<(), DockerError> {
        let output = Command::new("docker")
            .args(&["inspect", "-f", "{{.State.Running}}", container_id])
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .output()
            .await
            .map_err(|e| DockerError::CommandFailed(format!("Failed to inspect container: {}", e)))?;

        let is_running = String::from_utf8_lossy(&output.stdout).trim() == "true";

        if !is_running {
            info!("Container {} is not running, starting it", container_id);
            let output = Command::new("docker")
                .args(&["start", container_id])
                .stdout(Stdio::piped())
                .stderr(Stdio::piped())
                .output()
                .await
                .map_err(|e| DockerError::CommandFailed(format!("Failed to start container: {}", e)))?;

            if !output.status.success() {
                let stderr = String::from_utf8_lossy(&output.stderr);
                return Err(DockerError::CommandFailed(format!("Failed to start container: {}", stderr)));
            }

            info!("Container started successfully");
        } else {
            info!("Container is already running");
        }

        Ok(())
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
        ];

        // Add container name if persistent reuse is enabled
        let name_str;
        if config.reuse_container {
            if let Some(ref name) = config.container_name {
                args.push("--name");
                name_str = name.clone();
                args.push(&name_str);
            }
        } else {
            // Use --rm for temporary containers
            args.push("--rm");
        }

        args.extend_from_slice(&[
            "--memory",
            &config.memory_limit,
            "--cpus",
            &config.cpu_limit,
            "--network",
            &config.network_mode,
        ]);

        // Relax sandbox permissions for security tools that require raw sockets / capabilities.
        // This is needed for tools like nmap (e.g. ping scan) inside hardened runtimes.
        args.extend_from_slice(&[
            "--cap-add=NET_RAW",
            "--cap-add=NET_ADMIN",
            "--security-opt",
            "no-new-privileges=false",
        ]);

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
            
            // If container name already exists, try to remove and retry
            if stderr.contains("already in use") && config.reuse_container {
                if let Some(ref name) = config.container_name {
                    warn!("Container name already in use, removing old container: {}", name);
                    let _ = Command::new("docker")
                        .args(&["rm", "-f", name])
                        .output()
                        .await;
                    
                    // Retry creation
                    let retry_output = Command::new("docker")
                        .args(&args)
                        .stdout(Stdio::piped())
                        .stderr(Stdio::piped())
                        .output()
                        .await
                        .map_err(|e| DockerError::CommandFailed(format!("Failed to create container (retry): {}", e)))?;
                    
                    if !retry_output.status.success() {
                        let retry_stderr = String::from_utf8_lossy(&retry_output.stderr);
                        return Err(DockerError::CommandFailed(format!("Docker run failed (retry): {}", retry_stderr)));
                    }
                    
                    let container_id = String::from_utf8_lossy(&retry_output.stdout).trim().to_string();
                    info!("Container created (after retry): {}", container_id);
                    return Ok(container_id);
                }
            }
            
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

        // Execute command in container as root (for tools like nmap -sS that require privileges)
        let timeout_duration = Duration::from_secs(timeout_secs);
        let result = timeout(
            timeout_duration,
            Command::new("docker")
                .args(&["exec", "--user", "root", &container_id, "bash", "-c", command])
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

    /// Cleanup all containers in pool
    pub async fn cleanup_all() {
        let mut pool = CONTAINER_POOL.write().await;
        pool.cleanup_all().await;
    }

    /// Cleanup persistent shell container
    pub async fn cleanup_persistent_shell_container() -> Result<(), DockerError> {
        let config = DockerSandboxConfig::default();
        if let Some(container_name) = &config.container_name {
            info!("Cleaning up persistent shell container: {}", container_name);
            let output = Command::new("docker")
                .args(&["rm", "-f", container_name])
                .stdout(Stdio::piped())
                .stderr(Stdio::piped())
                .output()
                .await
                .map_err(|e| DockerError::CommandFailed(format!("Failed to remove container: {}", e)))?;

            if !output.status.success() {
                let stderr = String::from_utf8_lossy(&output.stderr);
                warn!("Failed to remove persistent shell container: {}", stderr);
            } else {
                info!("Persistent shell container removed successfully");
            }
        }
        Ok(())
    }

    /// Get persistent shell container info
    pub async fn get_persistent_shell_container_info() -> Result<Option<String>, DockerError> {
        let config = DockerSandboxConfig::default();
        ContainerPool::find_persistent_container(&config).await
    }

    /// Copy file from host to container
    pub async fn copy_file_to_container(
        &self,
        host_path: &str,
        container_path: &str,
    ) -> Result<(), DockerError> {
        // Ensure container is running
        let container_id = {
            let mut pool = CONTAINER_POOL.write().await;
            pool.get_container(&self.config).await?
        };

        info!("Copying file to container: {} -> {}:{}", host_path, container_id, container_path);

        // Create parent directory in container
        let parent_dir = std::path::Path::new(container_path)
            .parent()
            .and_then(|p| p.to_str())
            .unwrap_or("/workspace/uploads");
        
        let mkdir_output = Command::new("docker")
            .args(&["exec", &container_id, "mkdir", "-p", parent_dir])
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .output()
            .await
            .map_err(|e| DockerError::CommandFailed(format!("Failed to create directory: {}", e)))?;

        if !mkdir_output.status.success() {
            let stderr = String::from_utf8_lossy(&mkdir_output.stderr);
            warn!("Failed to create directory in container: {}", stderr);
        }

        // Copy file using docker cp
        let output = Command::new("docker")
            .args(&["cp", host_path, &format!("{}:{}", container_id, container_path)])
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .output()
            .await
            .map_err(|e| DockerError::CommandFailed(format!("Failed to copy file: {}", e)))?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            return Err(DockerError::CommandFailed(format!(
                "Docker cp failed: {}",
                stderr
            )));
        }

        info!("File copied successfully to container: {}", container_path);
        Ok(())
    }

    /// Copy file from container to host
    pub async fn copy_file_from_container(
        &self,
        container_path: &str,
        host_path: &str,
    ) -> Result<(), DockerError> {
        let container_id = {
            let mut pool = CONTAINER_POOL.write().await;
            pool.get_container(&self.config).await?
        };

        info!("Copying file from container: {}:{} -> {}", container_id, container_path, host_path);

        let output = Command::new("docker")
            .args(&["cp", &format!("{}:{}", container_id, container_path), host_path])
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .output()
            .await
            .map_err(|e| DockerError::CommandFailed(format!("Failed to copy file: {}", e)))?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            return Err(DockerError::CommandFailed(format!(
                "Docker cp failed: {}",
                stderr
            )));
        }

        Ok(())
    }

    /// Delete file in container
    pub async fn delete_file_in_container(&self, container_path: &str) -> Result<(), DockerError> {
        let container_id = {
            let mut pool = CONTAINER_POOL.write().await;
            pool.get_container(&self.config).await?
        };

        let output = Command::new("docker")
            .args(&["exec", &container_id, "rm", "-f", container_path])
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .output()
            .await
            .map_err(|e| DockerError::CommandFailed(format!("Failed to delete file: {}", e)))?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            warn!("Failed to delete file in container: {}", stderr);
        }

        Ok(())
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
