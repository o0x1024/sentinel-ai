//! Daemon process management for agent-browser
//!
//! Handles starting, stopping, and monitoring the Node.js daemon process.

use anyhow::{Context, Result};
use std::env;
use std::fs;
use std::path::PathBuf;
use std::process::{Child, Command, Stdio};
use tracing::{debug, info};

#[cfg(unix)]
use std::os::unix::net::UnixStream;
#[cfg(windows)]
use std::net::TcpStream;

/// Get the temp directory for daemon files
fn get_temp_dir() -> PathBuf {
    env::temp_dir()
}

/// Get socket path for Unix or port for Windows
pub fn get_socket_path(session: &str) -> String {
    #[cfg(unix)]
    {
        get_temp_dir()
            .join(format!("agent-browser-{}.sock", session))
            .to_string_lossy()
            .to_string()
    }
    #[cfg(windows)]
    {
        // Windows uses TCP port based on session hash
        let port = get_port_for_session(session);
        port.to_string()
    }
}

/// Get PID file path
pub fn get_pid_file(session: &str) -> PathBuf {
    get_temp_dir().join(format!("agent-browser-{}.pid", session))
}

/// Calculate port number from session name (Windows)
#[cfg(windows)]
fn get_port_for_session(session: &str) -> u16 {
    let mut hash: i32 = 0;
    for c in session.chars() {
        hash = hash.wrapping_shl(5).wrapping_sub(hash).wrapping_add(c as i32);
    }
    // Port range 49152-65535
    49152 + ((hash.abs() as u32) % 16383) as u16
}

/// Check if daemon is running for the session
pub fn is_daemon_running(session: &str) -> bool {
    let pid_file = get_pid_file(session);
    if !pid_file.exists() {
        return false;
    }

    match fs::read_to_string(&pid_file) {
        Ok(pid_str) => {
            if let Ok(pid) = pid_str.trim().parse::<u32>() {
                if is_process_alive(pid) {
                    return true;
                }
            }
            // Stale PID file, clean up
            cleanup_daemon_files(session);
            false
        }
        Err(_) => false,
    }
}

/// Check if a process is alive
fn is_process_alive(pid: u32) -> bool {
    #[cfg(unix)]
    {
        unsafe { libc::kill(pid as i32, 0) == 0 }
    }
    #[cfg(windows)]
    {
        use windows_sys::Win32::Foundation::CloseHandle;
        use windows_sys::Win32::System::Threading::{OpenProcess, PROCESS_QUERY_LIMITED_INFORMATION};
        unsafe {
            let handle = OpenProcess(PROCESS_QUERY_LIMITED_INFORMATION, 0, pid);
            if handle != std::ptr::null_mut() {
                CloseHandle(handle);
                true
            } else {
                false
            }
        }
    }
}

/// Clean up daemon files
pub fn cleanup_daemon_files(session: &str) {
    let pid_file = get_pid_file(session);
    let _ = fs::remove_file(&pid_file);

    #[cfg(unix)]
    {
        let socket_path = get_socket_path(session);
        let _ = fs::remove_file(&socket_path);
    }

    // Clean up stream port file
    let stream_file = get_temp_dir().join(format!("agent-browser-{}.stream", session));
    let _ = fs::remove_file(&stream_file);
}

/// Get the agent-browser daemon script path
fn get_daemon_script_path() -> Result<PathBuf> {
    // Try multiple locations
    let mut candidates = vec![];
    
    // Development: absolute path (for debugging)
    #[cfg(debug_assertions)]
    {
        candidates.push(PathBuf::from("/Users/a1024/code/ai/sentinel-ai/src-tauri/agent-browser/dist/daemon.js"));
    }
    
    // Release: bundled resources directory
    #[cfg(not(debug_assertions))]
    {
        // Try to get resource directory from Tauri
        if let Ok(exe_path) = env::current_exe() {
            if let Some(exe_dir) = exe_path.parent() {
                // macOS: Resources directory in app bundle
                #[cfg(target_os = "macos")]
                {
                    let resources_path = exe_dir.join("../Resources/agent-browser/dist/daemon.js");
                    candidates.push(resources_path);
                }
                // Windows: resources next to exe
                #[cfg(target_os = "windows")]
                {
                    let resources_path = exe_dir.join("agent-browser/dist/daemon.js");
                    candidates.push(resources_path);
                }
                // Linux: resources next to exe
                #[cfg(target_os = "linux")]
                {
                    let resources_path = exe_dir.join("agent-browser/dist/daemon.js");
                    candidates.push(resources_path);
                }
            }
        }
    }
    
    candidates.extend(vec![
        // Development: in src-tauri directory
        PathBuf::from("src-tauri/agent-browser/dist/daemon.js"),
        // Development: relative to workspace
        PathBuf::from("agent-browser/dist/daemon.js"),
        // Installed in node_modules
        PathBuf::from("node_modules/agent-browser/dist/daemon.js"),
        // User data directory (for manual installation)
        tauri_app_data_dir().join("agent-browser/dist/daemon.js"),
        // Global npm install
        home_dir().join(".npm-global/lib/node_modules/agent-browser/dist/daemon.js"),
    ]);

    for path in &candidates {
        debug!("Checking daemon path: {:?}", path);
        if path.exists() {
            info!("Found daemon at: {:?}", path);
            return Ok(path.clone());
        }
    }
    
    // Log current directory for debugging
    if let Ok(cwd) = env::current_dir() {
        debug!("Current working directory: {:?}", cwd);
    }

    // Try to find via which/where
    #[cfg(unix)]
    {
        if let Ok(output) = Command::new("which").arg("agent-browser").output() {
            if output.status.success() {
                let bin_path = String::from_utf8_lossy(&output.stdout).trim().to_string();
                // The binary is a wrapper, find the actual daemon.js
                let daemon_path = PathBuf::from(&bin_path)
                    .parent()
                    .and_then(|p| p.parent())
                    .map(|p| p.join("lib/node_modules/agent-browser/dist/daemon.js"));
                if let Some(path) = daemon_path {
                    if path.exists() {
                        return Ok(path);
                    }
                }
            }
        }
    }

    anyhow::bail!("agent-browser daemon not found. Please install it with: npm install -g agent-browser")
}

/// Get tauri app data directory
fn tauri_app_data_dir() -> PathBuf {
    #[cfg(target_os = "macos")]
    {
        home_dir().join("Library/Application Support/sentinel-ai")
    }
    #[cfg(target_os = "linux")]
    {
        home_dir().join(".local/share/sentinel-ai")
    }
    #[cfg(target_os = "windows")]
    {
        home_dir().join("AppData/Roaming/sentinel-ai")
    }
}

/// Get home directory
fn home_dir() -> PathBuf {
    dirs::home_dir().unwrap_or_else(|| PathBuf::from("."))
}

/// Daemon manager
pub struct DaemonManager {
    session: String,
    child: Option<Child>,
}

impl DaemonManager {
    pub fn new(session: &str) -> Self {
        Self {
            session: session.to_string(),
            child: None,
        }
    }

    /// Start the daemon if not already running
    pub fn start(&mut self) -> Result<()> {
        if is_daemon_running(&self.session) {
            info!("Daemon already running for session: {}", self.session);
            return Ok(());
        }

        info!("Starting agent-browser daemon for session: {}", self.session);

        // Clean up any stale files
        cleanup_daemon_files(&self.session);

        // Find daemon script
        let daemon_script = get_daemon_script_path()?;
        debug!("Using daemon script: {:?}", daemon_script);

        // Start the daemon process
        let mut cmd = Command::new("node");
        cmd.arg(&daemon_script)
            .env("AGENT_BROWSER_SESSION", &self.session)
            .env("AGENT_BROWSER_DAEMON", "1")
            .stdin(Stdio::null())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped());
        
        // Set Chrome executable path based on platform if not already set
        if env::var("AGENT_BROWSER_EXECUTABLE_PATH").is_err() {
            #[cfg(target_os = "macos")]
            {
                cmd.env("AGENT_BROWSER_EXECUTABLE_PATH", "/Applications/Google Chrome.app/Contents/MacOS/Google Chrome");
            }
            #[cfg(target_os = "windows")]
            {
                let chrome_paths = [
                    "C:\\Program Files\\Google\\Chrome\\Application\\chrome.exe",
                    "C:\\Program Files (x86)\\Google\\Chrome\\Application\\chrome.exe",
                ];
                for path in &chrome_paths {
                    if PathBuf::from(path).exists() {
                        cmd.env("AGENT_BROWSER_EXECUTABLE_PATH", path);
                        break;
                    }
                }
            }
            #[cfg(target_os = "linux")]
            {
                let chrome_paths = [
                    "/usr/bin/google-chrome",
                    "/usr/bin/google-chrome-stable",
                    "/usr/bin/chromium",
                    "/usr/bin/chromium-browser",
                ];
                for path in &chrome_paths {
                    if PathBuf::from(path).exists() {
                        cmd.env("AGENT_BROWSER_EXECUTABLE_PATH", path);
                        break;
                    }
                }
            }
        }

        let child = cmd.spawn().context("Failed to start agent-browser daemon")?;
        let pid = child.id();
        info!("Daemon started with PID: {}", pid);

        // Wait for daemon to be ready
        self.wait_for_ready()?;

        self.child = Some(child);
        Ok(())
    }

    /// Wait for daemon to be ready
    fn wait_for_ready(&self) -> Result<()> {
        let max_attempts = 50; // 5 seconds total
        let delay = std::time::Duration::from_millis(100);

        for attempt in 0..max_attempts {
            if self.can_connect() {
                debug!("Daemon ready after {} attempts", attempt + 1);
                return Ok(());
            }
            std::thread::sleep(delay);
        }

        anyhow::bail!("Daemon failed to start within timeout")
    }

    /// Check if we can connect to the daemon
    fn can_connect(&self) -> bool {
        #[cfg(unix)]
        {
            let socket_path = get_socket_path(&self.session);
            UnixStream::connect(&socket_path).is_ok()
        }
        #[cfg(windows)]
        {
            let port: u16 = get_socket_path(&self.session).parse().unwrap_or(0);
            TcpStream::connect(format!("127.0.0.1:{}", port)).is_ok()
        }
    }

    /// Stop the daemon
    pub fn stop(&mut self) -> Result<()> {
        info!("Stopping daemon for session: {}", self.session);

        // Try graceful shutdown via close command first
        // This will be handled by the client

        // Kill the process if we have a handle
        if let Some(mut child) = self.child.take() {
            let _ = child.kill();
            let _ = child.wait();
        }

        // Kill by PID if process is still running
        let pid_file = get_pid_file(&self.session);
        if let Ok(pid_str) = fs::read_to_string(&pid_file) {
            if let Ok(pid) = pid_str.trim().parse::<u32>() {
                kill_process(pid);
            }
        }

        // Clean up files
        cleanup_daemon_files(&self.session);

        Ok(())
    }

    /// Check if daemon is running
    pub fn is_running(&self) -> bool {
        is_daemon_running(&self.session)
    }

    /// Get session name
    pub fn session(&self) -> &str {
        &self.session
    }
}

impl Drop for DaemonManager {
    fn drop(&mut self) {
        // Don't stop daemon on drop - it should persist
        // The daemon will be stopped explicitly when needed
    }
}

/// Kill a process by PID
fn kill_process(pid: u32) {
    #[cfg(unix)]
    {
        unsafe {
            libc::kill(pid as i32, libc::SIGTERM);
        }
    }
    #[cfg(windows)]
    {
        use windows_sys::Win32::Foundation::CloseHandle;
        use windows_sys::Win32::System::Threading::{OpenProcess, TerminateProcess, PROCESS_TERMINATE};
        unsafe {
            let handle = OpenProcess(PROCESS_TERMINATE, 0, pid);
            if handle != std::ptr::null_mut() {
                TerminateProcess(handle, 1);
                CloseHandle(handle);
            }
        }
    }
}

/// Ensure daemon is running, start if needed
pub fn ensure_daemon(session: &str) -> Result<()> {
    if is_daemon_running(session) {
        return Ok(());
    }

    let mut manager = DaemonManager::new(session);
    manager.start()?;

    // Detach - daemon will continue running
    std::mem::forget(manager);

    Ok(())
}

/// Stop daemon for a session
pub fn stop_daemon(session: &str) {
    let pid_file = get_pid_file(session);
    if let Ok(content) = std::fs::read_to_string(&pid_file) {
        if let Ok(pid) = content.trim().parse::<u32>() {
            info!("Stopping agent-browser daemon (PID: {})", pid);
            kill_process(pid);
        }
    }
    cleanup_daemon_files(session);
}

/// Stop all daemon sessions
pub fn stop_all_daemons() {
    // Stop default session
    stop_daemon("default");
    
    // Clean up any stale files
    let tmp_dir = std::env::temp_dir();
    if let Ok(entries) = std::fs::read_dir(&tmp_dir) {
        for entry in entries.flatten() {
            let name = entry.file_name().to_string_lossy().to_string();
            if name.starts_with("agent-browser-") && name.ends_with(".pid") {
                // Extract session name
                let session = name
                    .strip_prefix("agent-browser-")
                    .and_then(|s| s.strip_suffix(".pid"))
                    .unwrap_or("default");
                stop_daemon(session);
            }
        }
    }
}
