//! Shell tool configuration commands

use sentinel_tools::shell::{
    get_shell_config, set_shell_config, ShellConfig,
};
use sentinel_tools::{
    init_docker_sandbox, DockerSandbox,
};
use serde::{Deserialize, Serialize};

/// Shell configuration response
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ShellConfigResponse {
    pub config: ShellConfig,
    pub docker_available: bool,
}

/// Get current shell configuration
#[tauri::command]
pub async fn get_shell_configuration() -> Result<ShellConfigResponse, String> {
    let config = get_shell_config().await;
    let docker_available = DockerSandbox::is_docker_available().await;
    
    Ok(ShellConfigResponse {
        config,
        docker_available,
    })
}

/// Update shell configuration
#[tauri::command]
pub async fn update_shell_configuration(config: ShellConfig) -> Result<(), String> {
    set_shell_config(config).await;
    Ok(())
}

/// Initialize Docker sandbox
#[tauri::command]
pub async fn initialize_docker_sandbox() -> Result<String, String> {
    // Get Dockerfile path from project root
    let dockerfile_path = std::env::current_dir()
        .map_err(|e| format!("Failed to get current directory: {}", e))?
        .join("sentinel-tools")
        .join("Dockerfile.sandbox");
    
    let dockerfile_str = dockerfile_path
        .to_str()
        .ok_or_else(|| "Invalid Dockerfile path".to_string())?;
    
    init_docker_sandbox(Some(dockerfile_str), Some("sentinel-sandbox:latest"))
        .await
        .map_err(|e| format!("Failed to initialize Docker sandbox: {}", e))?;
    
    Ok("Docker sandbox initialized successfully".to_string())
}

/// Check Docker availability
#[tauri::command]
pub async fn check_docker_availability() -> Result<bool, String> {
    Ok(DockerSandbox::is_docker_available().await)
}

/// Build Docker sandbox image
#[tauri::command]
pub async fn build_docker_sandbox_image() -> Result<String, String> {
    let dockerfile_path = std::env::current_dir()
        .map_err(|e| format!("Failed to get current directory: {}", e))?
        .join("sentinel-tools")
        .join("Dockerfile.sandbox");
    
    let dockerfile_str = dockerfile_path
        .to_str()
        .ok_or_else(|| "Invalid Dockerfile path".to_string())?;
    
    DockerSandbox::build_image(dockerfile_str, "sentinel-sandbox:latest")
        .await
        .map_err(|e| format!("Failed to build Docker image: {}", e))?;
    
    Ok("Docker image built successfully".to_string())
}

/// Cleanup all Docker containers in pool
#[tauri::command]
pub async fn cleanup_docker_containers() -> Result<String, String> {
    DockerSandbox::cleanup_all().await;
    Ok("All Docker containers cleaned up".to_string())
}

/// Cleanup persistent shell container
#[tauri::command]
pub async fn cleanup_shell_container() -> Result<String, String> {
    DockerSandbox::cleanup_persistent_shell_container()
        .await
        .map_err(|e| format!("Failed to cleanup shell container: {}", e))?;
    Ok("Shell container cleaned up".to_string())
}

/// Get shell container info
#[tauri::command]
pub async fn get_shell_container_info() -> Result<Option<String>, String> {
    DockerSandbox::get_persistent_shell_container_info()
        .await
        .map_err(|e| format!("Failed to get shell container info: {}", e))
}
