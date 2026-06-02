use std::sync::Arc;

use serde_json::{json, Value};
use tokio::sync::RwLock;

use sentinel_browser::adapter::traits::{
    BrowserBackend, ImageFormat, MouseButton, MouseEvent, MouseEventKind,
    Point, Rect, ScreenshotOpts, ScrollEvent, TabId,
};
use sentinel_browser::adapter::cdp_adapter::CdpBackend;
use sentinel_browser::automation::actions::BrowserActions;
use sentinel_browser::humanize::profile::{HumanProfile, HumanizationLevel};
use sentinel_browser::lifecycle::launcher::{BrowserProcess, LaunchConfig};
use sentinel_browser::page::element::{ElementSelector, ResolvedElement};

use super::browser_automation::{
    BrowserAction, BrowserAutomationArgs, BrowserAutomationError, BrowserAutomationHandler,
};
use crate::buildin_tools::shell::{get_shell_config, ShellExecutionMode};

pub struct BrowserAutomationState {
    actions: RwLock<Option<BrowserActions>>,
    backend: Arc<RwLock<CdpBackend>>,
    active_tab: RwLock<Option<String>>,
    _process: RwLock<Option<BrowserProcess>>,
    level: RwLock<HumanizationLevel>,
    mouse_pos: RwLock<Point>,
    /// Cached container IP for Docker mode
    container_host: RwLock<Option<String>>,
}

impl BrowserAutomationState {
    pub fn new() -> Self {
        Self {
            actions: RwLock::new(None),
            backend: Arc::new(RwLock::new(CdpBackend::new())),
            active_tab: RwLock::new(None),
            _process: RwLock::new(None),
            level: RwLock::new(HumanizationLevel::Human),
            mouse_pos: RwLock::new(Point { x: 0.0, y: 0.0 }),
            container_host: RwLock::new(None),
        }
    }

    async fn get_active_tab(&self) -> Result<TabId, BrowserAutomationError> {
        self.active_tab
            .read()
            .await
            .as_ref()
            .map(|id| TabId(id.clone()))
            .ok_or(BrowserAutomationError::NotConnected)
    }

    async fn is_docker_mode(&self) -> bool {
        let config = get_shell_config().await;
        config.default_execution_mode == ShellExecutionMode::Docker
    }

    /// Get the Docker container name from shell config
    async fn get_container_name(&self) -> String {
        let config = get_shell_config().await;
        config
            .docker_config
            .and_then(|c| c.container_name)
            .unwrap_or_else(|| "sentinel-sandbox-main".to_string())
    }

    /// Get the container's IP address via `docker inspect`
    async fn get_container_ip(container: &str) -> Result<String, BrowserAutomationError> {
        let output = tokio::process::Command::new("docker")
            .args([
                "inspect",
                "-f",
                "{{range.NetworkSettings.Networks}}{{.IPAddress}}{{end}}",
                container,
            ])
            .output()
            .await
            .map_err(|e| {
                BrowserAutomationError::ExecutionFailed(format!("docker inspect failed: {}", e))
            })?;

        if !output.status.success() {
            return Err(BrowserAutomationError::ExecutionFailed(format!(
                "docker inspect failed: {}",
                String::from_utf8_lossy(&output.stderr)
            )));
        }

        let ip = String::from_utf8_lossy(&output.stdout).trim().to_string();
        if ip.is_empty() {
            return Err(BrowserAutomationError::ExecutionFailed(
                "Container has no IP address. Is it running?".to_string(),
            ));
        }
        Ok(ip)
    }

    /// Internal port Chrome uses inside the container (not externally exposed)
    const CHROME_INTERNAL_PORT: u16 = 9333;

    /// Launch Chromium inside the Docker container via `docker exec -d`,
    /// then set up socat to relay from 0.0.0.0:external_port -> 127.0.0.1:internal_port.
    /// This is needed because Chromium 148+ ignores --remote-debugging-address=0.0.0.0
    /// and always binds to 127.0.0.1, making Docker port mapping ineffective.
    async fn launch_chromium_in_container(
        container: &str,
        external_port: u16,
    ) -> Result<(), BrowserAutomationError> {
        let internal_port = Self::CHROME_INTERNAL_PORT;

        // Launch Chrome on internal port
        let output = tokio::process::Command::new("docker")
            .args([
                "exec",
                "-d",
                container,
                "chromium",
                "--headless=new",
                &format!("--remote-debugging-port={}", internal_port),
                "--no-sandbox",
                "--disable-gpu",
                "--disable-dev-shm-usage",
                "--disable-extensions",
                "--disable-background-timer-throttling",
                "about:blank",
            ])
            .output()
            .await
            .map_err(|e| {
                BrowserAutomationError::ExecutionFailed(format!(
                    "Failed to exec chromium in container: {}",
                    e
                ))
            })?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            return Err(BrowserAutomationError::ExecutionFailed(format!(
                "chromium launch in container failed: {}",
                stderr
            )));
        }

        // Wait for Chrome to bind its internal port
        tokio::time::sleep(tokio::time::Duration::from_secs(2)).await;

        // Launch socat to relay: 0.0.0.0:external_port -> 127.0.0.1:internal_port
        let socat_args = format!(
            "TCP-LISTEN:{},fork,reuseaddr,bind=0.0.0.0 TCP:127.0.0.1:{}",
            external_port, internal_port
        );
        let output = tokio::process::Command::new("docker")
            .args(["exec", "-d", container, "socat"])
            .args(socat_args.split_whitespace())
            .output()
            .await
            .map_err(|e| {
                BrowserAutomationError::ExecutionFailed(format!(
                    "Failed to start socat relay in container: {}",
                    e
                ))
            })?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            return Err(BrowserAutomationError::ExecutionFailed(format!(
                "socat relay failed in container: {}. Install socat: apt-get install -y socat",
                stderr
            )));
        }

        Ok(())
    }

    /// Connect to browser, trying Docker or Host path based on execution mode.
    /// Returns the WebSocket URL on success.
    async fn connect_to_browser(
        &self,
        port: u16,
        headless: bool,
        proxy: Option<&str>,
    ) -> Result<String, BrowserAutomationError> {
        if self.is_docker_mode().await {
            self.connect_docker(port).await
        } else {
            self.connect_host(port, headless, proxy).await
        }
    }

    /// Docker path: ensure chromium is running in container, connect via published port or container IP
    async fn connect_docker(&self, port: u16) -> Result<String, BrowserAutomationError> {
        let container = self.get_container_name().await;

        // Try published port first (works on macOS + Linux)
        // Then fall back to container IP (Linux only, for older containers without -p)
        let host = self.resolve_docker_host(&container, port).await;

        // Check if chromium is already running
        if let Ok(version) =
            sentinel_browser::lifecycle::discovery::get_browser_version_at(&host, port).await
        {
            tracing::info!("Chromium already running, reachable at {}:{}", host, port);
            *self.container_host.write().await = Some(host);
            return Ok(version.web_socket_debugger_url);
        }

        // Launch chromium + socat relay in container
        tracing::info!("Launching chromium in container '{}'...", container);
        Self::launch_chromium_in_container(&container, port).await?;

        // Wait for chromium to be ready (retry up to 10s)
        for attempt in 0..20 {
            tokio::time::sleep(tokio::time::Duration::from_millis(500)).await;
            if let Ok(version) =
                sentinel_browser::lifecycle::discovery::get_browser_version_at(&host, port).await
            {
                tracing::info!(
                    "Chromium ready in container after {}ms",
                    (attempt + 1) * 500
                );
                *self.container_host.write().await = Some(host);
                return Ok(version.web_socket_debugger_url);
            }
        }

        Err(BrowserAutomationError::ExecutionFailed(format!(
            "Chromium launched in container '{}' but DevTools not reachable at {}:{}. \
             If the container was created without -p 9222:9222, please recreate it.",
            container, host, port
        )))
    }

    /// Determine the best host to reach the container's CDP port.
    /// Prefers published port (127.0.0.1), falls back to container IP on Linux.
    async fn resolve_docker_host(&self, container: &str, port: u16) -> String {
        // Try 127.0.0.1 first (port is published via -p 9222:9222)
        if sentinel_browser::lifecycle::discovery::get_browser_version_at("127.0.0.1", port)
            .await
            .is_ok()
        {
            return "127.0.0.1".to_string();
        }

        // Check if port is published by querying docker port
        let port_check = tokio::process::Command::new("docker")
            .args(["port", container, &format!("{}", port)])
            .output()
            .await;

        if let Ok(output) = port_check {
            if output.status.success() {
                let mapping = String::from_utf8_lossy(&output.stdout);
                if !mapping.trim().is_empty() {
                    // Port is published, use localhost
                    return "127.0.0.1".to_string();
                }
            }
        }

        // Fallback: try container IP directly (works on Linux, not macOS)
        if let Ok(ip) = Self::get_container_ip(container).await {
            return ip;
        }

        "127.0.0.1".to_string()
    }

    /// Host path: try existing Chrome, or launch one
    async fn connect_host(&self, port: u16, headless: bool, proxy: Option<&str>) -> Result<String, BrowserAutomationError> {
        // Try existing Chrome
        if let Ok(version) =
            sentinel_browser::lifecycle::discovery::get_browser_version(port).await
        {
            return Ok(version.web_socket_debugger_url);
        }

        // Launch Chrome locally
        tracing::info!("No Chrome on port {}, launching locally...", port);
        let config = LaunchConfig {
            headless,
            debugging_port: port,
            proxy: proxy.map(|s| s.to_string()),
            ..Default::default()
        };

        let process = BrowserProcess::launch_chrome(&config).map_err(|e| {
            BrowserAutomationError::ExecutionFailed(format!(
                "Failed to launch Chrome: {}. Ensure Chrome/Chromium is installed.",
                e
            ))
        })?;
        *self._process.write().await = Some(process);

        // Wait for Chrome to be ready
        for attempt in 0..20 {
            tokio::time::sleep(tokio::time::Duration::from_millis(500)).await;
            if let Ok(version) =
                sentinel_browser::lifecycle::discovery::get_browser_version(port).await
            {
                tracing::info!("Chrome ready locally after {}ms", (attempt + 1) * 500);
                return Ok(version.web_socket_debugger_url);
            }
        }

        Err(BrowserAutomationError::ExecutionFailed(
            "Chrome launched but DevTools not reachable. \
             Try: chromium --headless=new --remote-debugging-port=9222 \
             --remote-debugging-address=127.0.0.1 --no-sandbox"
                .to_string(),
        ))
    }

    async fn ensure_connected(&self) -> Result<(), BrowserAutomationError> {
        {
            let backend = self.backend.read().await;
            if backend.is_connected() {
                return Ok(());
            }
        }

        let port = 9222u16;

        // Determine host to probe based on execution mode
        let host = if self.is_docker_mode().await {
            // Try cached container IP or re-discover
            let cached = self.container_host.read().await.clone();
            if let Some(ip) = cached {
                ip
            } else {
                let container = self.get_container_name().await;
                match Self::get_container_ip(&container).await {
                    Ok(ip) => {
                        *self.container_host.write().await = Some(ip.clone());
                        ip
                    }
                    Err(_) => "127.0.0.1".to_string(),
                }
            }
        } else {
            "127.0.0.1".to_string()
        };

        // Try to connect
        if let Ok(version) =
            sentinel_browser::lifecycle::discovery::get_browser_version_at(&host, port).await
        {
            let mut backend = self.backend.write().await;
            if backend
                .connect(&version.web_socket_debugger_url)
                .await
                .is_ok()
            {
                drop(backend);
                self.init_actions_and_tabs().await;
                tracing::info!("Browser auto-connected at {}:{}", host, port);
                return Ok(());
            }
        }

        Err(BrowserAutomationError::NotConnected)
    }

    /// Initialize BrowserActions and set active tab after connection
    async fn init_actions_and_tabs(&self) {
        let level = *self.level.read().await;
        let profile = HumanProfile::Casual;
        let actions = BrowserActions::new(
            self.backend.clone() as Arc<RwLock<dyn BrowserBackend>>,
            profile,
            level,
        );
        *self.actions.write().await = Some(actions);

        let backend = self.backend.read().await;
        if let Ok(tabs) = backend.list_tabs().await {
            if let Some(tab) = tabs.first() {
                *self.active_tab.write().await = Some(tab.id.0.clone());
            }
        }
    }

    async fn resolve_tab(&self, args: &BrowserAutomationArgs) -> Result<TabId, BrowserAutomationError> {
        if let Some(ref id) = args.tab_id {
            Ok(TabId(id.clone()))
        } else {
            self.get_active_tab().await
        }
    }

    fn parse_humanize_level(s: &str) -> HumanizationLevel {
        match s {
            "raw" => HumanizationLevel::Raw,
            "basic" => HumanizationLevel::Basic,
            "human" => HumanizationLevel::Human,
            "stealth" => HumanizationLevel::Stealth,
            _ => HumanizationLevel::Human,
        }
    }
}

#[async_trait::async_trait]
impl BrowserAutomationHandler for BrowserAutomationState {
    async fn handle(
        &self,
        args: BrowserAutomationArgs,
    ) -> Result<Value, BrowserAutomationError> {
        match args.action {
            BrowserAction::Launch => self.handle_launch(&args).await,
            BrowserAction::Navigate => self.handle_navigate(&args).await,
            BrowserAction::PageState => self.handle_page_state(&args).await,
            BrowserAction::Click => self.handle_click(&args).await,
            BrowserAction::Type => self.handle_type(&args).await,
            BrowserAction::Scroll => self.handle_scroll(&args).await,
            BrowserAction::Wait => self.handle_wait(&args).await,
            BrowserAction::Eval => self.handle_eval(&args).await,
            BrowserAction::Screenshot => self.handle_screenshot(&args).await,
            BrowserAction::Tabs => self.handle_tabs(&args).await,
            BrowserAction::Cookies => self.handle_cookies(&args).await,
            BrowserAction::Network => self.handle_network(&args).await,
            BrowserAction::Close => self.handle_close(&args).await,
        }
    }
}

impl BrowserAutomationState {
    async fn handle_launch(&self, args: &BrowserAutomationArgs) -> Result<Value, BrowserAutomationError> {
        let headless = args.headless.unwrap_or(true);
        let port = 9222u16;

        let level = args
            .humanize
            .as_deref()
            .map(Self::parse_humanize_level)
            .unwrap_or(HumanizationLevel::Human);
        *self.level.write().await = level;

        // Unified launch: Docker or Host path determined automatically
        let ws_url = self.connect_to_browser(port, headless, args.proxy.as_deref()).await?;

        // Connect via WebSocket
        {
            let mut backend = self.backend.write().await;
            backend
                .connect(&ws_url)
                .await
                .map_err(|e| BrowserAutomationError::ExecutionFailed(format!(
                    "WebSocket connection failed: {}", e
                )))?;
        }

        // Initialize BrowserActions with humanization
        self.init_actions_and_tabs().await;

        // Get tab count for response
        let backend = self.backend.read().await;
        let tab_count = backend
            .list_tabs()
            .await
            .map(|t| t.len())
            .unwrap_or(0);

        let mode = if self.is_docker_mode().await { "docker" } else { "host" };

        Ok(json!({
            "status": "connected",
            "mode": mode,
            "tabs": tab_count,
            "humanize_level": format!("{:?}", level),
        }))
    }

    async fn handle_navigate(&self, args: &BrowserAutomationArgs) -> Result<Value, BrowserAutomationError> {
        self.ensure_connected().await?;
        let tab = self.resolve_tab(args).await?;
        let url = args.url.as_deref().unwrap_or("");

        let mut actions_guard = self.actions.write().await;
        if let Some(ref mut actions) = *actions_guard {
            actions
                .navigate(&tab, url)
                .await
                .map_err(|e| BrowserAutomationError::ExecutionFailed(format!("{}", e)))?;
        } else {
            let backend = self.backend.read().await;
            backend
                .navigate(&tab, url)
                .await
                .map_err(|e| BrowserAutomationError::ExecutionFailed(format!("{}", e)))?;
        }

        // Get final URL
        let backend = self.backend.read().await;
        let current_url = backend
            .evaluate_js(&tab, "window.location.href")
            .await
            .ok()
            .and_then(|v| v.as_str().map(|s| s.to_string()))
            .unwrap_or_else(|| url.to_string());

        Ok(json!({
            "url": current_url,
            "navigated": true,
        }))
    }

    async fn handle_page_state(&self, args: &BrowserAutomationArgs) -> Result<Value, BrowserAutomationError> {
        self.ensure_connected().await?;
        let tab = self.resolve_tab(args).await?;
        let mode = args.mode.as_deref().unwrap_or("a11y_tree");
        let backend = self.backend.read().await;

        match mode {
            "a11y_tree" | "accessibility_tree" => {
                let tree = backend
                    .get_accessibility_tree(&tab)
                    .await
                    .map_err(|e| BrowserAutomationError::ExecutionFailed(format!("{}", e)))?;
                Ok(json!({
                    "mode": "a11y_tree",
                    "content": tree.to_text(),
                }))
            }
            "screenshot" => {
                let data = backend
                    .capture_screenshot(&tab, ScreenshotOpts {
                        full_page: args.full_page.unwrap_or(false),
                        format: ImageFormat::Png,
                        quality: None,
                        clip: None,
                    })
                    .await
                    .map_err(|e| BrowserAutomationError::ExecutionFailed(format!("{}", e)))?;
                use base64::Engine;
                let b64 = base64::engine::general_purpose::STANDARD.encode(&data);
                Ok(json!({
                    "mode": "screenshot",
                    "format": "png",
                    "size_bytes": data.len(),
                    "base64": b64,
                }))
            }
            "dom" => {
                let html = backend
                    .evaluate_js(&tab, "document.documentElement.outerHTML")
                    .await
                    .map_err(|e| BrowserAutomationError::ExecutionFailed(format!("{}", e)))?;
                Ok(json!({ "mode": "dom", "content": html }))
            }
            _ => Err(BrowserAutomationError::InvalidInput(format!(
                "Unknown mode: '{}'. Use 'a11y_tree', 'screenshot', or 'dom'",
                mode
            ))),
        }
    }

    async fn handle_click(&self, args: &BrowserAutomationArgs) -> Result<Value, BrowserAutomationError> {
        self.ensure_connected().await?;
        let tab = self.resolve_tab(args).await?;
        let selector = args.selector.as_deref().unwrap_or("");

        // Resolve element to get bounds
        let element = self.resolve_element(&tab, selector).await?;

        let mut actions_guard = self.actions.write().await;
        if let Some(ref mut actions) = *actions_guard {
            actions
                .click(&tab, &element)
                .await
                .map_err(|e| BrowserAutomationError::ExecutionFailed(format!("{}", e)))?;
        } else {
            // Fallback: direct click at center
            let center = element.center();
            let backend = self.backend.read().await;
            backend
                .dispatch_mouse_event(&tab, MouseEvent {
                    kind: MouseEventKind::Pressed,
                    x: center.x,
                    y: center.y,
                    button: MouseButton::Left,
                })
                .await
                .map_err(|e| BrowserAutomationError::ExecutionFailed(format!("{}", e)))?;
            backend
                .dispatch_mouse_event(&tab, MouseEvent {
                    kind: MouseEventKind::Released,
                    x: center.x,
                    y: center.y,
                    button: MouseButton::Left,
                })
                .await
                .map_err(|e| BrowserAutomationError::ExecutionFailed(format!("{}", e)))?;
        }

        // Track mouse position
        let click_pt = element.random_click_point();
        *self.mouse_pos.write().await = click_pt.clone();

        Ok(json!({
            "clicked": selector,
            "coordinates": [click_pt.x, click_pt.y],
        }))
    }

    async fn handle_type(&self, args: &BrowserAutomationArgs) -> Result<Value, BrowserAutomationError> {
        self.ensure_connected().await?;
        let tab = self.resolve_tab(args).await?;
        let selector = args.selector.as_deref().unwrap_or("");
        let text = args.text.as_deref().unwrap_or("");

        // Focus the element first
        let backend = self.backend.read().await;
        let focus_js = format!(
            r#"(() => {{ const el = {}; if(el) {{ el.focus(); return true; }} return false; }})()"#,
            self.selector_to_js(selector)
        );
        backend
            .evaluate_js(&tab, &focus_js)
            .await
            .map_err(|e| BrowserAutomationError::ExecutionFailed(format!("{}", e)))?;

        if args.clear_first.unwrap_or(false) {
            backend
                .evaluate_js(&tab, &format!(
                    r#"(() => {{ const el = {}; if(el) {{ el.value = ''; el.dispatchEvent(new Event('input', {{bubbles:true}})); }} }})()"#,
                    self.selector_to_js(selector)
                ))
                .await
                .map_err(|e| BrowserAutomationError::ExecutionFailed(format!("{}", e)))?;
        }
        drop(backend);

        // Type with humanization via BrowserActions
        let mut actions_guard = self.actions.write().await;
        if let Some(ref mut actions) = *actions_guard {
            actions
                .type_text(&tab, text)
                .await
                .map_err(|e| BrowserAutomationError::ExecutionFailed(format!("{}", e)))?;
        } else {
            // Fallback: raw typing
            use sentinel_browser::adapter::traits::{KeyEvent, KeyEventKind, KeyModifiers};
            let backend = self.backend.read().await;
            for ch in text.chars() {
                backend
                    .dispatch_key_event(&tab, KeyEvent {
                        kind: KeyEventKind::Char,
                        key: ch.to_string(),
                        code: String::new(),
                        modifiers: KeyModifiers::default(),
                    })
                    .await
                    .map_err(|e| BrowserAutomationError::ExecutionFailed(format!("{}", e)))?;
            }
        }

        Ok(json!({
            "typed": text,
            "selector": selector,
            "chars": text.len(),
        }))
    }

    async fn handle_scroll(&self, args: &BrowserAutomationArgs) -> Result<Value, BrowserAutomationError> {
        self.ensure_connected().await?;
        let tab = self.resolve_tab(args).await?;

        let direction = args.direction.as_deref().unwrap_or("down");
        let amount = args.amount.unwrap_or(300.0);

        let delta_y = match direction {
            "up" => -amount,
            "down" => amount,
            _ => amount,
        };

        let mut actions_guard = self.actions.write().await;
        if let Some(ref mut actions) = *actions_guard {
            actions
                .scroll(&tab, delta_y)
                .await
                .map_err(|e| BrowserAutomationError::ExecutionFailed(format!("{}", e)))?;
        } else {
            let backend = self.backend.read().await;
            backend
                .dispatch_scroll_event(&tab, ScrollEvent {
                    x: 400.0,
                    y: 400.0,
                    delta_x: 0.0,
                    delta_y,
                })
                .await
                .map_err(|e| BrowserAutomationError::ExecutionFailed(format!("{}", e)))?;
        }

        Ok(json!({ "scrolled": direction, "amount": amount }))
    }

    async fn handle_wait(&self, args: &BrowserAutomationArgs) -> Result<Value, BrowserAutomationError> {
        let wait_for = args.wait_for.as_deref().unwrap_or("time");
        let timeout = args.timeout_ms.unwrap_or(5000);

        match wait_for {
            "time" => {
                tokio::time::sleep(tokio::time::Duration::from_millis(timeout)).await;
                Ok(json!({ "waited_ms": timeout }))
            }
            "element" => {
                self.ensure_connected().await?;
                let tab = self.resolve_tab(args).await?;
                let selector = args.selector.as_deref().unwrap_or("");
                let backend = self.backend.read().await;

                let start = std::time::Instant::now();
                loop {
                    let js = format!("!!{}", self.selector_to_js(selector));
                    if let Ok(Value::Bool(true)) = backend.evaluate_js(&tab, &js).await {
                        return Ok(json!({
                            "found": true,
                            "selector": selector,
                            "elapsed_ms": start.elapsed().as_millis(),
                        }));
                    }
                    if start.elapsed().as_millis() as u64 > timeout {
                        return Err(BrowserAutomationError::Timeout(format!(
                            "Element '{}' not found within {}ms", selector, timeout
                        )));
                    }
                    tokio::time::sleep(tokio::time::Duration::from_millis(200)).await;
                }
            }
            "network_idle" => {
                tokio::time::sleep(tokio::time::Duration::from_millis(timeout.min(3000))).await;
                Ok(json!({ "waited_for": "network_idle" }))
            }
            _ => {
                tokio::time::sleep(tokio::time::Duration::from_millis(timeout)).await;
                Ok(json!({ "waited_ms": timeout }))
            }
        }
    }

    async fn handle_eval(&self, args: &BrowserAutomationArgs) -> Result<Value, BrowserAutomationError> {
        self.ensure_connected().await?;
        let tab = self.resolve_tab(args).await?;
        let expression = args.expression.as_deref().unwrap_or("");

        let backend = self.backend.read().await;
        let result = backend
            .evaluate_js(&tab, expression)
            .await
            .map_err(|e| BrowserAutomationError::ExecutionFailed(format!("{}", e)))?;

        Ok(json!({ "result": result }))
    }

    async fn handle_screenshot(&self, args: &BrowserAutomationArgs) -> Result<Value, BrowserAutomationError> {
        self.ensure_connected().await?;
        let tab = self.resolve_tab(args).await?;
        let format_str = args.format.as_deref().unwrap_or("png");
        let full_page = args.full_page.unwrap_or(false);

        let format = match format_str {
            "jpeg" | "jpg" => ImageFormat::Jpeg,
            "webp" => ImageFormat::Webp,
            _ => ImageFormat::Png,
        };

        let backend = self.backend.read().await;
        let data = backend
            .capture_screenshot(&tab, ScreenshotOpts {
                full_page,
                format,
                quality: if matches!(format, ImageFormat::Jpeg) { Some(80) } else { None },
                clip: None,
            })
            .await
            .map_err(|e| BrowserAutomationError::ExecutionFailed(format!("{}", e)))?;

        use base64::Engine;
        let b64 = base64::engine::general_purpose::STANDARD.encode(&data);

        Ok(json!({
            "format": format_str,
            "size_bytes": data.len(),
            "base64": b64,
        }))
    }

    async fn handle_tabs(&self, args: &BrowserAutomationArgs) -> Result<Value, BrowserAutomationError> {
        self.ensure_connected().await?;
        let tab_action = args.tab_action.as_deref().unwrap_or("list");
        let backend = self.backend.read().await;

        match tab_action {
            "list" => {
                let tabs = backend
                    .list_tabs()
                    .await
                    .map_err(|e| BrowserAutomationError::ExecutionFailed(format!("{}", e)))?;
                let active = self.active_tab.read().await;
                let tab_list: Vec<Value> = tabs
                    .iter()
                    .map(|t| json!({
                        "id": t.id.0,
                        "url": t.url,
                        "title": t.title,
                        "active": active.as_deref() == Some(&t.id.0),
                    }))
                    .collect();
                Ok(json!({ "tabs": tab_list }))
            }
            "create" => {
                let url = args.url.as_deref().unwrap_or("about:blank");
                let new_tab = backend
                    .create_tab(Some(url))
                    .await
                    .map_err(|e| BrowserAutomationError::ExecutionFailed(format!("{}", e)))?;
                *self.active_tab.write().await = Some(new_tab.0.clone());
                Ok(json!({ "created": new_tab.0 }))
            }
            "close" => {
                let tab = self.resolve_tab(args).await?;
                backend
                    .close_tab(&tab)
                    .await
                    .map_err(|e| BrowserAutomationError::ExecutionFailed(format!("{}", e)))?;
                Ok(json!({ "closed": tab.0 }))
            }
            "switch" => {
                let tab_id = args.tab_id.as_deref().ok_or(
                    BrowserAutomationError::InvalidInput("switch requires 'tab_id'".to_string()),
                )?;
                backend
                    .activate_tab(&TabId(tab_id.to_string()))
                    .await
                    .map_err(|e| BrowserAutomationError::ExecutionFailed(format!("{}", e)))?;
                *self.active_tab.write().await = Some(tab_id.to_string());
                Ok(json!({ "switched_to": tab_id }))
            }
            _ => Err(BrowserAutomationError::InvalidInput(format!(
                "Unknown tab_action: '{}'", tab_action
            ))),
        }
    }

    async fn handle_cookies(&self, args: &BrowserAutomationArgs) -> Result<Value, BrowserAutomationError> {
        self.ensure_connected().await?;
        let tab = self.resolve_tab(args).await?;
        let backend = self.backend.read().await;

        let cookies = backend
            .get_cookies(&tab)
            .await
            .map_err(|e| BrowserAutomationError::ExecutionFailed(format!("{}", e)))?;

        Ok(json!({
            "cookies": cookies.iter().map(|c| json!({
                "name": c.name,
                "value": c.value,
                "domain": c.domain,
            })).collect::<Vec<_>>(),
        }))
    }

    async fn handle_network(&self, args: &BrowserAutomationArgs) -> Result<Value, BrowserAutomationError> {
        self.ensure_connected().await?;
        let tab = self.resolve_tab(args).await?;
        let patterns = args.patterns.as_deref().unwrap_or(&[]);
        let backend = self.backend.read().await;

        backend
            .intercept_requests(&tab, patterns)
            .await
            .map_err(|e| BrowserAutomationError::ExecutionFailed(format!("{}", e)))?;

        Ok(json!({ "interception_enabled": true, "patterns": patterns }))
    }

    async fn handle_close(&self, _args: &BrowserAutomationArgs) -> Result<Value, BrowserAutomationError> {
        *self.actions.write().await = None;

        let mut backend = self.backend.write().await;
        backend
            .disconnect()
            .await
            .map_err(|e| BrowserAutomationError::ExecutionFailed(format!("{}", e)))?;

        *self.active_tab.write().await = None;

        Ok(json!({ "status": "disconnected" }))
    }

    // --- Helper methods ---

    fn selector_to_js(&self, selector: &str) -> String {
        if selector.starts_with('e') && selector[1..].parse::<u32>().is_ok() {
            format!(
                "document.querySelectorAll('[data-sentinel-ref=\"{}\"]')[0] || document.querySelectorAll('*')[{}]",
                selector,
                selector[1..].parse::<usize>().unwrap_or(0)
            )
        } else if selector.starts_with("text:") {
            let text = &selector[5..];
            format!(
                r#"Array.from(document.querySelectorAll('a,button,input,label,[role="button"],[role="link"]')).find(el => el.textContent.trim().includes('{}'))"#,
                text.replace('\'', "\\'")
            )
        } else {
            format!("document.querySelector('{}')", selector.replace('\'', "\\'"))
        }
    }

    async fn resolve_element(
        &self,
        tab: &TabId,
        selector: &str,
    ) -> Result<ResolvedElement, BrowserAutomationError> {
        let backend = self.backend.read().await;

        let js = format!(
            r#"(() => {{
                const el = {};
                if (!el) return null;
                const rect = el.getBoundingClientRect();
                const style = window.getComputedStyle(el);
                return {{
                    x: rect.left,
                    y: rect.top,
                    width: rect.width,
                    height: rect.height,
                    visible: style.display !== 'none' && style.visibility !== 'hidden' && style.opacity !== '0',
                    tag: el.tagName.toLowerCase(),
                    disabled: el.disabled === true,
                }};
            }})()"#,
            self.selector_to_js(selector)
        );

        let result = backend
            .evaluate_js(tab, &js)
            .await
            .map_err(|e| BrowserAutomationError::ExecutionFailed(format!("{}", e)))?;

        match result {
            Value::Object(ref obj) => {
                let x = obj.get("x").and_then(|v| v.as_f64()).unwrap_or(0.0);
                let y = obj.get("y").and_then(|v| v.as_f64()).unwrap_or(0.0);
                let width = obj.get("width").and_then(|v| v.as_f64()).unwrap_or(0.0);
                let height = obj.get("height").and_then(|v| v.as_f64()).unwrap_or(0.0);
                let visible = obj.get("visible").and_then(|v| v.as_bool()).unwrap_or(true);
                let tag = obj.get("tag").and_then(|v| v.as_str()).unwrap_or("").to_string();
                let disabled = obj.get("disabled").and_then(|v| v.as_bool()).unwrap_or(false);

                Ok(ResolvedElement {
                    selector: if selector.starts_with("text:") {
                        ElementSelector::Text(selector[5..].to_string())
                    } else if selector.starts_with('e') && selector[1..].parse::<u32>().is_ok() {
                        ElementSelector::RefId(selector.to_string())
                    } else {
                        ElementSelector::Css(selector.to_string())
                    },
                    bounds: Rect { x, y, width, height },
                    is_visible: visible,
                    is_in_viewport: x >= 0.0 && y >= 0.0,
                    is_interactable: visible && !disabled && width > 0.0 && height > 0.0,
                    tag_name: tag,
                    attributes: vec![],
                })
            }
            Value::Null => Err(BrowserAutomationError::ElementNotFound(format!(
                "Element '{}' not found on page", selector
            ))),
            _ => Err(BrowserAutomationError::ExecutionFailed(
                "Unexpected element resolve result".to_string(),
            )),
        }
    }
}
