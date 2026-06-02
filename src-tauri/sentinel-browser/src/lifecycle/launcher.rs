use std::path::PathBuf;
use std::process::Child;

use crate::adapter::traits::BrowserError;

/// Configuration for launching a browser instance
#[derive(Debug, Clone)]
pub struct LaunchConfig {
    /// Path to browser executable (auto-detected if None)
    pub browser_path: Option<PathBuf>,
    /// User data directory for persistent state (cookies, localStorage, etc.)
    pub user_data_dir: Option<PathBuf>,
    /// Remote debugging port (default: 9222 for CDP, pipe for Juggler)
    pub debugging_port: u16,
    /// Run in headless mode
    pub headless: bool,
    /// Proxy server URL
    pub proxy: Option<String>,
    /// Window size
    pub window_size: Option<(u32, u32)>,
    /// Additional command-line arguments
    pub extra_args: Vec<String>,
    /// Backend type
    pub backend: BrowserBackendType,
}

#[derive(Debug, Clone, Copy)]
pub enum BrowserBackendType {
    ChromeCdp,
    CamoufoxJuggler,
}

impl Default for LaunchConfig {
    fn default() -> Self {
        Self {
            browser_path: None,
            user_data_dir: None,
            debugging_port: 9222,
            headless: false,
            proxy: None,
            window_size: Some((1920, 1080)),
            extra_args: vec![],
            backend: BrowserBackendType::ChromeCdp,
        }
    }
}

pub struct BrowserProcess {
    pub child: Child,
    pub endpoint: String,
    pub config: LaunchConfig,
}

impl BrowserProcess {
    /// Launch Chrome with remote debugging enabled
    pub fn launch_chrome(config: &LaunchConfig) -> Result<Self, BrowserError> {
        let browser_path = config
            .browser_path
            .clone()
            .unwrap_or_else(|| detect_chrome_path());

        let mut args = vec![
            format!("--remote-debugging-port={}", config.debugging_port),
            "--remote-debugging-address=127.0.0.1".to_string(),
            "--no-first-run".to_string(),
            "--no-default-browser-check".to_string(),
            "--disable-background-timer-throttling".to_string(),
            "--disable-backgrounding-occluded-windows".to_string(),
            "--disable-renderer-backgrounding".to_string(),
            "--disable-extensions".to_string(),
            "--disable-component-extensions-with-background-pages".to_string(),
            "--disable-default-apps".to_string(),
        ];

        if config.headless {
            args.push("--headless=new".to_string());
        }

        // In Docker/container environments, Chrome requires --no-sandbox
        if std::env::var("CHROME_FLAGS").is_ok() || std::path::Path::new("/.dockerenv").exists() {
            args.push("--no-sandbox".to_string());
            args.push("--disable-dev-shm-usage".to_string());
            args.push("--disable-gpu".to_string());
        }

        if let Some(ref proxy) = config.proxy {
            args.push(format!("--proxy-server={}", proxy));
        }

        if let Some((w, h)) = config.window_size {
            args.push(format!("--window-size={},{}", w, h));
        }

        if let Some(ref data_dir) = config.user_data_dir {
            args.push(format!("--user-data-dir={}", data_dir.display()));
        }

        args.extend(config.extra_args.clone());

        // Chrome needs an initial URL to fully bootstrap in headless mode
        args.push("about:blank".to_string());

        let mut child = std::process::Command::new(&browser_path)
            .args(&args)
            .stdin(std::process::Stdio::null())
            .stdout(std::process::Stdio::null())
            .stderr(std::process::Stdio::piped())
            .spawn()
            .map_err(|e| BrowserError::ConnectionFailed(format!(
                "Failed to launch Chrome at {:?}: {}", browser_path, e
            )))?;

        // Drain stderr in background to prevent pipe buffer from blocking Chrome
        if let Some(stderr) = child.stderr.take() {
            std::thread::spawn(move || {
                use std::io::Read;
                let mut buf = [0u8; 4096];
                let mut reader = std::io::BufReader::new(stderr);
                loop {
                    match reader.read(&mut buf) {
                        Ok(0) | Err(_) => break,
                        Ok(_) => {}
                    }
                }
            });
        }

        Ok(Self {
            child,
            endpoint: format!("ws://127.0.0.1:{}", config.debugging_port),
            config: config.clone(),
        })
    }
}

impl BrowserProcess {
    /// Launch Camoufox (Firefox-based) with Juggler protocol via stdio pipes.
    /// Returns a BrowserProcess with stdin/stdout available for pipe communication.
    pub fn launch_camoufox(config: &LaunchConfig) -> Result<Self, BrowserError> {
        let browser_path = config
            .browser_path
            .clone()
            .unwrap_or_else(|| detect_camoufox_path());

        let mut args = vec![
            "--juggler-pipe".to_string(),
            "--no-remote".to_string(),
        ];

        if config.headless {
            args.push("--headless".to_string());
        }

        if let Some(ref proxy) = config.proxy {
            args.push(format!("--proxy-server={}", proxy));
        }

        if let Some((w, h)) = config.window_size {
            args.push(format!("--width={}", w));
            args.push(format!("--height={}", h));
        }

        if let Some(ref data_dir) = config.user_data_dir {
            args.push("--profile".to_string());
            args.push(data_dir.display().to_string());
        }

        args.extend(config.extra_args.clone());

        let child = std::process::Command::new(&browser_path)
            .args(&args)
            .stdin(std::process::Stdio::piped())
            .stdout(std::process::Stdio::piped())
            .stderr(std::process::Stdio::null())
            .spawn()
            .map_err(|e| BrowserError::ConnectionFailed(format!(
                "Failed to launch Camoufox at {:?}: {}", browser_path, e
            )))?;

        Ok(Self {
            child,
            endpoint: "pipe://juggler".to_string(),
            config: config.clone(),
        })
    }
}

impl Drop for BrowserProcess {
    fn drop(&mut self) {
        let _ = self.child.kill();
    }
}

fn detect_camoufox_path() -> PathBuf {
    #[cfg(target_os = "macos")]
    {
        let paths = [
            "/Applications/Camoufox.app/Contents/MacOS/camoufox",
            "/usr/local/bin/camoufox",
        ];
        for p in paths {
            if std::path::Path::new(p).exists() {
                return PathBuf::from(p);
            }
        }
    }

    #[cfg(target_os = "linux")]
    {
        let paths = [
            "/usr/bin/camoufox",
            "/opt/camoufox/camoufox",
        ];
        for p in paths {
            if std::path::Path::new(p).exists() {
                return PathBuf::from(p);
            }
        }
    }

    PathBuf::from("camoufox")
}

fn detect_chrome_path() -> PathBuf {
    // Check CHROME_BIN env var first (Docker / CI environments)
    if let Ok(chrome_bin) = std::env::var("CHROME_BIN") {
        let p = PathBuf::from(&chrome_bin);
        if p.exists() {
            return p;
        }
    }

    #[cfg(target_os = "macos")]
    {
        let paths = [
            "/Applications/Google Chrome.app/Contents/MacOS/Google Chrome",
            "/Applications/Chromium.app/Contents/MacOS/Chromium",
        ];
        for p in paths {
            if std::path::Path::new(p).exists() {
                return PathBuf::from(p);
            }
        }
    }

    #[cfg(target_os = "linux")]
    {
        let paths = [
            "/usr/bin/google-chrome",
            "/usr/bin/chromium-browser",
            "/usr/bin/chromium",
        ];
        for p in paths {
            if std::path::Path::new(p).exists() {
                return PathBuf::from(p);
            }
        }
    }

    #[cfg(target_os = "windows")]
    {
        let paths = [
            r"C:\Program Files\Google\Chrome\Application\chrome.exe",
            r"C:\Program Files (x86)\Google\Chrome\Application\chrome.exe",
        ];
        for p in paths {
            if std::path::Path::new(p).exists() {
                return PathBuf::from(p);
            }
        }
    }

    PathBuf::from("chrome")
}
