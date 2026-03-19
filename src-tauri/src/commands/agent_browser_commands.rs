//! Agent Browser commands for Tauri

use serde::{Deserialize, Serialize};
use std::process::Command;
use std::path::PathBuf;
use tauri::{AppHandle, Manager, command, path::BaseDirectory};

#[derive(Debug, Serialize, Deserialize)]
pub struct AgentBrowserOutput {
    pub success: bool,
    pub stdout: String,
    pub stderr: String,
    pub exit_code: i32,
}

/// Run agent-browser command with arguments
/// 
/// # Examples
/// ```rust
/// run_agent_browser_command(app_handle, vec!["open".to_string(), "https://example.com".to_string()])
/// ```
#[command]
pub async fn run_agent_browser_command(
    app_handle: AppHandle,
    args: Vec<String>,
) -> Result<AgentBrowserOutput, String> {
    // Get the sidecar/bin path
    let agent_browser_path = get_agent_browser_path(&app_handle)?;

    // Ensure the binary exists
    if !agent_browser_path.exists() {
        return Err(format!(
            "agent-browser binary not found at: {}",
            agent_browser_path.display()
        ));
    }

    // Execute the command
    let output = Command::new(&agent_browser_path)
        .args(&args)
        .output()
        .map_err(|e| format!("Failed to execute agent-browser: {}", e))?;

    let stdout = String::from_utf8_lossy(&output.stdout).to_string();
    let stderr = String::from_utf8_lossy(&output.stderr).to_string();
    let exit_code = output.status.code().unwrap_or(-1);

    Ok(AgentBrowserOutput {
        success: output.status.success(),
        stdout,
        stderr,
        exit_code,
    })
}

/// Get agent-browser version
#[command]
pub async fn get_agent_browser_version(
    app_handle: AppHandle,
) -> Result<String, String> {
    let agent_browser_path = get_agent_browser_path(&app_handle)?;

    if !agent_browser_path.exists() {
        return Err(format!(
            "agent-browser binary not found at: {}",
            agent_browser_path.display()
        ));
    }

    let output = Command::new(&agent_browser_path)
        .arg("--version")
        .output()
        .map_err(|e| format!("Failed to get agent-browser version: {}", e))?;

    if output.status.success() {
        Ok(String::from_utf8_lossy(&output.stdout).trim().to_string())
    } else {
        Err(String::from_utf8_lossy(&output.stderr).to_string())
    }
}

/// Helper function to get the agent-browser binary path
fn get_agent_browser_path(app_handle: &AppHandle) -> Result<PathBuf, String> {
    let target_suffix = if cfg!(target_os = "macos") && cfg!(target_arch = "aarch64") {
        "aarch64-apple-darwin"
    } else if cfg!(target_os = "macos") && cfg!(target_arch = "x86_64") {
        "x86_64-apple-darwin"
    } else if cfg!(target_os = "linux") && cfg!(target_arch = "aarch64") {
        "aarch64-unknown-linux-gnu"
    } else if cfg!(target_os = "linux") && cfg!(target_arch = "x86_64") {
        "x86_64-unknown-linux-gnu"
    } else if cfg!(target_os = "windows") {
        "x86_64-pc-windows-msvc"
    } else {
        ""
    };

    let binary_name = if cfg!(windows) {
        "sentinel-agent-browser.exe"
    } else {
        "sentinel-agent-browser"
    };
    let sidecar_name = if cfg!(windows) {
        format!("sentinel-agent-browser-{target_suffix}.exe")
    } else {
        format!("sentinel-agent-browser-{target_suffix}")
    };

    let legacy_binary_name = if cfg!(windows) {
        "agent-browser.exe"
    } else {
        "agent-browser"
    };

    let legacy_sidecar_name = if cfg!(windows) {
        format!("agent-browser-{target_suffix}.exe")
    } else {
        format!("agent-browser-{target_suffix}")
    };

    let mut common_paths = vec![];

    if let Ok(path) = app_handle
        .path()
        .resolve("bin/sentinel-agent-browser", BaseDirectory::Resource)
    {
        common_paths.push(path);
    }
    if let Ok(path) = app_handle.path().resolve(format!("bin/{sidecar_name}"), BaseDirectory::Resource) {
        common_paths.push(path);
    }
    if let Ok(path) = app_handle
        .path()
        .resolve("bin/agent-browser", BaseDirectory::Resource)
    {
        common_paths.push(path);
    }
    if let Ok(path) = app_handle
        .path()
        .resolve(format!("bin/{legacy_sidecar_name}"), BaseDirectory::Resource)
    {
        common_paths.push(path);
    }

    common_paths.extend([
        PathBuf::from("src-tauri/bin").join(binary_name),
        PathBuf::from("src-tauri/bin").join(&sidecar_name),
        PathBuf::from("src-tauri/bin").join(legacy_binary_name),
        PathBuf::from("src-tauri/bin").join(&legacy_sidecar_name),
        PathBuf::from(binary_name),
        PathBuf::from(legacy_binary_name),
    ]);

    for path in common_paths {
        if path.exists() {
            return Ok(path);
        }
    }

    // Try which command (Unix-like systems)
    if !cfg!(windows) {
        if let Ok(output) = Command::new("which")
            .arg("agent-browser")
            .output()
        {
            if output.status.success() {
                let path = String::from_utf8_lossy(&output.stdout)
                    .trim()
                    .to_string();
                if !path.is_empty() {
                    return Ok(PathBuf::from(path));
                }
            }
        }
    }

    Err(
        "agent-browser binary not found. Make sure it's installed or run 'npm run setup:agent-browser'"
            .to_string(),
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_binary_name() {
        let name = if cfg!(windows) {
            "agent-browser.exe"
        } else {
            "agent-browser"
        };
        assert_eq!(name.is_empty(), false);
    }
}
