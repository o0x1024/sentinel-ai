//! Daemon process management for agent-browser
//!
//! Handles starting, stopping, and monitoring the agent-browser daemon process.

use anyhow::{Context, Result};
use std::env;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Child, Command, Stdio};
use tracing::{debug, info};

#[cfg(windows)]
use std::net::TcpStream;
#[cfg(unix)]
use std::os::unix::net::UnixStream;

/// Get the temp directory for daemon files
fn get_temp_dir() -> PathBuf {
    env::temp_dir()
}

/// Get socket directory used by daemon
fn get_socket_dir() -> PathBuf {
    env::var("AGENT_BROWSER_SOCKET_DIR")
        .map(PathBuf::from)
        .unwrap_or_else(|_| get_temp_dir())
}

/// Get socket path for Unix or port for Windows
pub fn get_socket_path(session: &str) -> String {
    #[cfg(unix)]
    {
        get_socket_dir()
            .join(format!("{}.sock", session))
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
        hash = hash
            .wrapping_shl(5)
            .wrapping_sub(hash)
            .wrapping_add(c as i32);
    }
    // Port range 49152-65535
    49152 + ((hash.abs() as u32) % 16383) as u16
}

/// Check if daemon is running for the session
pub fn is_daemon_running(session: &str) -> bool {
    if can_connect_to_daemon(session) {
        return true;
    }

    let pid_file = get_pid_file(session);
    if !pid_file.exists() {
        return false;
    }

    match fs::read_to_string(&pid_file) {
        Ok(pid_str) => {
            if let Ok(pid) = pid_str.trim().parse::<u32>() {
                if is_process_alive(pid) {
                    for _ in 0..3 {
                        std::thread::sleep(std::time::Duration::from_millis(100));
                        if can_connect_to_daemon(session) {
                            return true;
                        }
                    }

                    debug!(
                        "Daemon PID {} is alive but socket is unreachable for session {}",
                        pid, session
                    );
                    cleanup_daemon_files(session);
                    return false;
                }
            }
            // Stale PID file, clean up
            cleanup_daemon_files(session);
            false
        }
        Err(_) => false,
    }
}

fn can_connect_to_daemon(session: &str) -> bool {
    #[cfg(unix)]
    {
        UnixStream::connect(get_socket_path(session)).is_ok()
    }
    #[cfg(windows)]
    {
        let port: u16 = get_socket_path(session).parse().unwrap_or(0);
        TcpStream::connect(format!("127.0.0.1:{}", port)).is_ok()
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
        use windows_sys::Win32::System::Threading::{
            OpenProcess, PROCESS_QUERY_LIMITED_INFORMATION,
        };
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
        let legacy_socket_path = get_temp_dir().join(format!("agent-browser-{}.sock", session));
        let _ = fs::remove_file(&legacy_socket_path);
    }

    // Clean up stream port file
    let stream_file = get_temp_dir().join(format!("agent-browser-{}.stream", session));
    let _ = fs::remove_file(&stream_file);
}

/// Resolve agent-browser daemon executable path
fn get_daemon_binary_path() -> Result<PathBuf> {
    if let Ok(explicit) = env::var("AGENT_BROWSER_BIN_PATH") {
        let path = PathBuf::from(explicit);
        if path.exists() {
            return Ok(path);
        }
    }

    let mut candidates = Vec::new();

    let binary_name = if cfg!(windows) {
        "sentinel-agent-browser.exe"
    } else {
        "sentinel-agent-browser"
    };

    let legacy_name = if cfg!(windows) {
        "agent-browser.exe"
    } else {
        "agent-browser"
    };

    let manifest_dir = Path::new(env!("CARGO_MANIFEST_DIR"));
    candidates.push(manifest_dir.join("../bin").join(binary_name));
    candidates.push(manifest_dir.join("../bin").join(legacy_name));

    if let Ok(exe_path) = env::current_exe() {
        if let Some(exe_dir) = exe_path.parent() {
            candidates.push(exe_dir.join(binary_name));
            candidates.push(exe_dir.join(legacy_name));
            #[cfg(target_os = "macos")]
            {
                candidates.push(exe_dir.join("../Resources").join(binary_name));
                candidates.push(exe_dir.join("../Resources").join(legacy_name));
            }
        }
    }

    for path in &candidates {
        debug!("Checking daemon binary path: {:?}", path);
        if path.exists() {
            info!("Found daemon binary at: {:?}", path);
            return Ok(path.clone());
        }
    }

    let which_cmd = if cfg!(windows) { "where" } else { "which" };
    for name in [binary_name, legacy_name] {
        if let Ok(output) = Command::new(which_cmd).arg(name).output() {
            if output.status.success() {
                let path = String::from_utf8_lossy(&output.stdout)
                    .lines()
                    .next()
                    .unwrap_or_default()
                    .trim()
                    .to_string();
                if !path.is_empty() {
                    let candidate = PathBuf::from(path);
                    if candidate.exists() {
                        return Ok(candidate);
                    }
                }
            }
        }
    }

    anyhow::bail!(
        "agent-browser daemon binary not found. Run `npm run setup:agent-browser` in project root."
    )
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

        info!(
            "Starting agent-browser daemon for session: {}",
            self.session
        );

        // Clean up any stale files
        cleanup_daemon_files(&self.session);

        // Find daemon binary
        let daemon_binary = get_daemon_binary_path()?;
        debug!("Using daemon binary: {:?}", daemon_binary);

        // Start the daemon process
        let mut cmd = Command::new(&daemon_binary);
        cmd.env("AGENT_BROWSER_SOCKET_DIR", get_socket_dir())
            .env("AGENT_BROWSER_SESSION", &self.session)
            .env("AGENT_BROWSER_DAEMON", "1")
            .stdin(Stdio::null())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped());

        // Set Chrome executable path based on platform if not already set
        if env::var("AGENT_BROWSER_EXECUTABLE_PATH").is_err() {
            #[cfg(target_os = "macos")]
            {
                cmd.env(
                    "AGENT_BROWSER_EXECUTABLE_PATH",
                    "/Applications/Google Chrome.app/Contents/MacOS/Google Chrome",
                );
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

        let mut child = cmd
            .spawn()
            .context("Failed to start agent-browser daemon")?;
        let pid = child.id();
        info!("Daemon started with PID: {}", pid);
        let _ = fs::write(get_pid_file(&self.session), pid.to_string());

        // Wait for daemon to be ready
        if let Err(err) = self.wait_for_ready() {
            let status = child.try_wait().ok().flatten();
            let _ = child.kill();
            let output = child.wait_with_output().ok();
            cleanup_daemon_files(&self.session);

            let stderr = output
                .as_ref()
                .map(|o| String::from_utf8_lossy(&o.stderr).trim().to_string())
                .filter(|s| !s.is_empty());
            let stdout = output
                .as_ref()
                .map(|o| String::from_utf8_lossy(&o.stdout).trim().to_string())
                .filter(|s| !s.is_empty());

            let mut details = Vec::new();
            details.push(format!("session={}", self.session));
            details.push(format!("pid={}", pid));
            if let Some(status) = status {
                details.push(format!("status={}", status));
            }
            if let Some(stderr) = stderr {
                details.push(format!("stderr={}", stderr));
            }
            if let Some(stdout) = stdout {
                details.push(format!("stdout={}", stdout));
            }

            return Err(err)
                .context(format!("agent-browser daemon startup failed ({})", details.join(", ")));
        }

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
        use windows_sys::Win32::System::Threading::{
            OpenProcess, TerminateProcess, PROCESS_TERMINATE,
        };
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

    cleanup_daemon_files(session);

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
