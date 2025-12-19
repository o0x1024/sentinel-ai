//! Worker Agent for scoped exploration
//!
//! Responsibilities:
//! - Explore assigned scope with strict boundary enforcement
//! - Report discovered APIs to global state
//! - Report cross-scope links for Manager coordination
//! - Smart context filtering with modal isolation

use super::element_filter::{format_filtered_for_prompt, ElementFilter, ElementFilterConfig};
use super::global_state::GlobalExplorerState;
use super::types::*;
use crate::engines::vision_explorer::action_builder;
use crate::engines::vision_explorer::integrations::TakeoverManager;
use crate::engines::vision_explorer::login_detector;
use crate::engines::vision_explorer::message_emitter::VisionExplorerMessageEmitter;
use crate::engines::vision_explorer::message_emitter::{WorkerActionInfo, WorkerDecisionInfo};
use crate::engines::vision_explorer::tools::BrowserTools;
use crate::engines::vision_explorer::types::{
    ActionResult, AnnotatedElement, BrowserAction, LoginField, PageState, TakeoverStatus, VlmAnalysisResult,
};
use crate::engines::vision_explorer::vlm_parser;
use crate::engines::{LlmClient, LlmConfig};
use anyhow::{anyhow, Result};
use sentinel_llm::MessageImageAttachment as ImageAttachment;
use std::collections::HashSet;
use std::sync::Arc;
use std::time::Instant;
use tokio::sync::RwLock;
use tracing::{debug, info, warn};

/// Worker system prompt template (fixed rules / output schema, higher priority than user prompt)
const WORKER_SYSTEM_PROMPT_TEMPLATE: &str = r#"You are a web exploration agent assigned to a specific section of a website.

## Your Assignment
Scope: {scope_name}
Context: {task_context}
Entry URL: {entry_url}
URL Patterns: {url_patterns}
Max Depth: {max_depth}

## Exploration Rules
1. ONLY interact with elements within your assigned scope
2. Skip links pointing OUTSIDE your scope patterns
3. Focus on: pages, forms, API endpoints, interactive features
4. ⚠️ MODAL PRIORITY: If a modal/dialog is active, you MUST interact with it first (close or complete it) before accessing background elements
5. Use go_back when stuck or when you've fully explored a sub-section

## Available Actions
- click_by_index: Click an element by its [index]
- fill_by_index: Fill an input field (index + value)
- hover_by_index: Hover over an element to reveal hidden menus/dropdowns (use for hamburger menus, nav triggers, expandable items)
- scroll: Scroll the page (direction: up/down)
- go_back: Navigate back to previous page
- press_escape: Close modal/dropdown by pressing Escape
- set_status: Mark exploration as "completed" when done

Respond with JSON:
```json
{
  "page_analysis": "Brief analysis of current page",
  "next_action": {
    "action_type": "click_by_index|fill_by_index|hover_by_index|scroll|go_back|press_escape|set_status",
    "element_index": 123,
    "value": "optional value for fill or scroll direction",
    "reason": "Why this action"
  },
  "estimated_apis": ["GET /api/example"],
  "exploration_progress": 0.5,
  "is_exploration_complete": false,
  "completion_reason": null
}
```"#;

/// Worker user prompt template (dynamic state / page content)
const WORKER_USER_PROMPT_TEMPLATE: &str = r#"## Current State
Iteration: {iteration} / {max_iterations}
Pages visited: {pages_visited}
APIs discovered: {apis_discovered}
Navigation path: {navigation_path}
Scroll status: {scroll_status}

## Recent Actions
{action_history}

## Current Page
URL: {current_url}
Title: {current_title}
{page_summary}

## Page Elements
{elements}"#;

/// Worker Agent for scoped exploration
pub struct WorkerAgent {
    /// Task assignment
    task: WorkerTask,
    /// Browser tools
    browser_tools: Arc<BrowserTools>,
    /// LLM configuration
    llm_config: LlmConfig,
    /// Global shared state
    global_state: Arc<GlobalExplorerState>,
    /// Enable multimodal (screenshot)
    enable_multimodal: bool,
    /// Local visited URLs (for scope tracking)
    local_visited: HashSet<String>,
    /// Discovered APIs
    discovered_apis: Vec<DiscoveredApi>,
    /// Cross-scope links found
    cross_scope_links: Vec<CrossScopeLink>,
    /// Current depth
    current_depth: u32,
    /// Action history
    action_history: Vec<String>,
    /// Iteration count
    iteration: u32,
    /// Start time
    start_time: Option<Instant>,
    /// Elements interacted count
    elements_interacted: usize,
    /// Navigation path stack (for go_back)
    navigation_path: Vec<String>,
    /// Last scroll position for detecting scroll exhaustion
    last_scroll_pos: f64,
    /// Consecutive same-position scrolls (to detect bottom)
    stuck_scroll_count: u32,
    /// Element filter for context optimization
    element_filter: ElementFilter,
    /// Takeover manager for human-in-the-loop interactions
    takeover_manager: Option<Arc<RwLock<TakeoverManager>>>,
    /// Last synced auth timestamp (for identifying updates)
    last_auth_timestamp: i64,
    /// Message emitter for frontend communication
    message_emitter: Option<Arc<VisionExplorerMessageEmitter>>,
}

impl WorkerAgent {
    /// Create new Worker Agent
    pub fn new(
        task: WorkerTask,
        browser_tools: Arc<BrowserTools>,
        llm_config: LlmConfig,
        global_state: Arc<GlobalExplorerState>,
        enable_multimodal: bool,
        takeover_manager: Option<Arc<RwLock<TakeoverManager>>>,
    ) -> Self {
        // Configure element filter with modal isolation enabled
        let filter_config = ElementFilterConfig {
            max_elements: 80,
            max_per_folded_region: 5,
            enable_modal_isolation: true,
            enable_pattern_dedup: true,
            similar_items_threshold: 5,
            viewport_width: 1280,
            viewport_height: 720,
        };

        Self {
            task,
            browser_tools,
            llm_config,
            global_state,
            enable_multimodal,
            local_visited: HashSet::new(),
            discovered_apis: Vec::new(),
            cross_scope_links: Vec::new(),
            current_depth: 0,
            action_history: Vec::new(),
            iteration: 0,
            start_time: None,
            elements_interacted: 0,
            navigation_path: Vec::new(),
            last_scroll_pos: 0.0,
            stuck_scroll_count: 0,
            element_filter: ElementFilter::new(filter_config),
            takeover_manager,
            last_auth_timestamp: 0,
            message_emitter: None,
        }
    }

    /// Set message emitter for frontend communication
    pub fn with_message_emitter(mut self, emitter: Arc<VisionExplorerMessageEmitter>) -> Self {
        self.message_emitter = Some(emitter);
        self
    }

    /// Set message emitter (mutable reference version)
    pub fn set_message_emitter(&mut self, emitter: Arc<VisionExplorerMessageEmitter>) {
        self.message_emitter = Some(emitter);
    }

    /// Execute the exploration task
    pub async fn execute(&mut self) -> Result<WorkerResult> {
        info!(
            "WorkerAgent [{}]: Starting exploration of scope '{}'",
            self.task.id, self.task.scope.name
        );
        self.start_time = Some(Instant::now());

        // Initial sync of auth state before starting work
        if let Err(e) = self.sync_auth_state().await {
            warn!("WorkerAgent [{}]: Failed to sync initial auth state: {}", self.task.id, e);
        }

        // Special handling for login task
        let is_login_task = self.task.scope.id == "login";
        
        // Build entry URL for use in navigation and fallback
        let entry_url = self.build_full_url(&self.task.scope.entry_url);

        // For login task, skip navigation - browser is already on login page from Manager analysis
        // This avoids duplicate navigation and potential state reset
        if is_login_task {
            info!(
                "WorkerAgent [{}]: Login task - skipping navigation, browser already on login page",
                self.task.id
            );
        } else {
            // Navigate to entry URL for non-login tasks
            let can_visit = self.global_state.try_visit_url(&entry_url).await;
            if !can_visit {
                // try_visit_url returns false for multiple reasons:
                // - already visited (normal in sequential / re-plan flows)
                // - ignored/blacklisted/outside-domain (should skip)
                if self.global_state.is_visited(&entry_url).await {
                    info!(
                        "WorkerAgent [{}]: Entry URL already visited globally, continuing exploration",
                        self.task.id
                    );

                    // Best-effort: if current page is already in scope, avoid a hard navigate which
                    // could reset SPA state. Otherwise navigate to entry_url as a safe fallback.
                    if let Ok(ps) = self.browser_tools.capture_page_state_text_mode().await {
                        if self.is_in_scope(&ps.url) {
                            self.local_visited.insert(ps.url);
                        } else {
                            let nav_action = BrowserAction::Navigate {
                                url: entry_url.clone(),
                            };
                            self.browser_tools.execute_action(&nav_action).await?;
                            self.local_visited.insert(entry_url.clone());
                            tokio::time::sleep(tokio::time::Duration::from_millis(500)).await;
                        }
                    } else {
                        // Fallback: navigate if we can't read current page state
                        let nav_action = BrowserAction::Navigate {
                            url: entry_url.clone(),
                        };
                        self.browser_tools.execute_action(&nav_action).await?;
                        self.local_visited.insert(entry_url.clone());
                        tokio::time::sleep(tokio::time::Duration::from_millis(500)).await;
                    }
                } else {
                    info!(
                        "WorkerAgent [{}]: Entry URL not eligible for exploration (ignored/blacklisted/outside domain), skipping",
                        self.task.id
                    );
                    return self.build_result("Entry URL not eligible for exploration");
                }
            } else {
                let nav_action = BrowserAction::Navigate {
                    url: entry_url.clone(),
                };
                self.browser_tools.execute_action(&nav_action).await?;
                self.local_visited.insert(entry_url.clone());

                tokio::time::sleep(tokio::time::Duration::from_millis(500)).await;
            }

            // === Post-navigation auth sync ===
            // Cookies/localStorage must be set on the correct domain.
            // Now that we've navigated to the target domain, re-apply auth state
            // and refresh the page to make cookies take effect.
            if let Some(auth_state) = self.global_state.get_auth_state().await {
                if auth_state.timestamp > 0 && (!auth_state.cookies.is_empty() || !auth_state.local_storage.is_empty()) {
                    info!(
                        "WorkerAgent [{}]: Applying cached auth state ({} cookies, {} localStorage items)",
                        self.task.id, auth_state.cookies.len(), auth_state.local_storage.len()
                    );
                    if let Err(e) = self.browser_tools.apply_auth_state(&auth_state).await {
                        warn!("WorkerAgent [{}]: Failed to apply auth state: {}", self.task.id, e);
                    } else {
                        // Refresh the page to apply cookies
                        let refresh_action = BrowserAction::Navigate {
                            url: entry_url.clone(),
                        };
                        if let Err(e) = self.browser_tools.execute_action(&refresh_action).await {
                            warn!("WorkerAgent [{}]: Failed to refresh after auth: {}", self.task.id, e);
                        }
                        tokio::time::sleep(tokio::time::Duration::from_millis(500)).await;
                        self.last_auth_timestamp = auth_state.timestamp;
                    }
                }
            }
        }

        // If this is a login task, pause and wait for user to manually login
        if is_login_task {
            let result = self.execute_login_handover().await;
            // If successful login, capture state
            if let Ok(ref res) = result {
                if res.completion_reason.contains("logged in") {
                     let _ = self.capture_and_broadcast_auth().await;
                }
            }
            return result;
        }

        // ==== Pre-exploration: Detect and click hamburger menu / navigation triggers ====
        // Many dashboard UIs have collapsed sidebars by default. This step tries to expand them.
        if let Err(e) = self.try_expand_hidden_navigation().await {
            warn!("WorkerAgent [{}]: Failed to expand hidden navigation: {}", self.task.id, e);
        }

        // Main exploration loop
        let mut completion_reason = "Max iterations reached".to_string();

        while self.iteration < self.task.max_iterations {
            self.iteration += 1;

            // Sync auth state at start of iteration
            if let Err(e) = self.sync_auth_state().await {
                warn!("WorkerAgent [{}]: Failed to sync auth state: {}", self.task.id, e);
            }

            // Check for pause
            self.global_state.wait_if_paused().await;

            // Capture page state
            let page_state = if self.enable_multimodal {
                self.browser_tools.capture_page_state().await?
            } else {
                self.browser_tools.capture_page_state_text_mode().await?
            };

            // Check if still in scope
            if !self.is_in_scope(&page_state.url) {
                warn!(
                    "WorkerAgent [{}]: Navigated outside scope to {}, going back",
                    self.task.id, page_state.url
                );
                // Record as cross-scope
                self.record_cross_scope(&page_state.url);
                // Go back
                if let Err(e) = self
                    .browser_tools
                    .execute_action(&BrowserAction::Navigate {
                        url: entry_url.clone(),
                    })
                    .await
                {
                    warn!("Failed to navigate back: {}", e);
                }
                continue;
            }

            // Mark visited
            self.global_state.mark_visited(&page_state.url).await;
            self.local_visited.insert(page_state.url.clone());

            self.global_state.mark_visited(&page_state.url).await;
            self.local_visited.insert(page_state.url.clone());

            // Check for login page presence (Scenario 2: discovered login page)
            if let Some(fields) = login_detector::detect_login_page(&page_state) {
                if let Some(tm) = &self.takeover_manager {
                    let mut tm_write = tm.write().await;
                    // Only request if not already skipped or detected
                    if !tm_write.is_login_skipped() && !tm_write.is_login_detected() {
                        if tm_write.request_login_takeover("Login page detected during exploration", Some(fields)) {
                            // Loop and wait for user credentials or manual login
                            drop(tm_write); // Release lock
                            
                            info!("WorkerAgent [{}]: Entered login wait loop", self.task.id);
                            
                            let mut login_success = false;
                            loop {
                                tokio::time::sleep(tokio::time::Duration::from_millis(1000)).await;
                                
                                // Check status
                                let tm_read = tm.read().await;
                                let status = tm_read.get_status();
                                
                                if matches!(status, TakeoverStatus::Returned | TakeoverStatus::Active) {
                                    // Resume exploration
                                    // Logic will pick up credentials if set, or just continue if manual login
                                    info!("WorkerAgent [{}]: Resuming from login wait (status={:?})", self.task.id, status);
                                    login_success = true;
                                    break;
                                }
                                
                                // Check cancellation
                                if self.global_state.is_paused().await { // Or check cancellation token if we had one
                                     // Just wait
                                }
                            }

                            // If resumed successfully, capture auth state
                            if login_success {
                                let _ = self.capture_and_broadcast_auth().await;
                            }
                        }
                    }
                }
            }

            // Extract and filter elements
            let (in_scope, out_scope) = self.partition_elements(&page_state.annotated_elements);

            // Record out-of-scope links
            for el in &out_scope {
                if let Some(href) = el.attributes.get("href") {
                    self.record_cross_scope(href);
                }
            }

            // Build prompts and call LLM
            let system_prompt = self.build_system_prompt();
            let user_prompt = self.build_user_prompt(&page_state, &in_scope).await?;
            let response = self
                .call_llm(&system_prompt, &user_prompt, page_state.screenshot.as_deref())
                .await?;

            // Parse response
            let analysis =
                match vlm_parser::parse_vlm_response(&response, 0, self.enable_multimodal) {
                    Ok(a) => a,
                    Err(e) => {
                        warn!(
                            "WorkerAgent [{}]: Failed to parse VLM response: {}",
                            self.task.id, e
                        );
                        continue;
                    }
                };

            // Emit decision (analysis -> next action) to frontend activity feed
            if let Some(emitter) = &self.message_emitter {
                let progress = (analysis.exploration_progress * 100.0).clamp(0.0, 100.0);
                let estimated_apis = if analysis.estimated_apis.is_empty() {
                    None
                } else {
                    Some(analysis.estimated_apis.clone())
                };

                emitter.emit_worker_decision(&WorkerDecisionInfo {
                    task_id: self.task.id.clone(),
                    scope_name: self.task.scope.name.clone(),
                    iteration: self.iteration,
                    page_analysis: analysis.page_analysis.clone(),
                    action_type: analysis.next_action.action_type.clone(),
                    element_index: analysis.next_action.element_index,
                    value: analysis.next_action.value.clone(),
                    reason: analysis.next_action.reason.clone(),
                    progress,
                    estimated_apis,
                });
            }

            // Record estimated APIs
            for api_str in &analysis.estimated_apis {
                if let Some(api) = parse_api_string(api_str) {
                    if self.global_state.register_api(api.clone()).await {
                        self.discovered_apis.push(api);
                    }
                }
            }

            // Check completion
            if analysis.is_exploration_complete {
                completion_reason = analysis
                    .completion_reason
                    .unwrap_or_else(|| "Worker decided exploration complete".to_string());
                break;
            }

            // Execute action
            let action = self.build_action(&analysis, &in_scope)?;
            let action_desc = format!("{:?}", action);

            let action_start = Instant::now();
            match self.browser_tools.execute_action(&action).await {
                Ok(result) => {
                    self.record_action(&action_desc, result.success);
                    if result.success {
                        self.elements_interacted += 1;
                    }

                    if let Some(emitter) = &self.message_emitter {
                        let duration_ms = Some(action_start.elapsed().as_millis() as u64);
                        emitter.emit_worker_action(&WorkerActionInfo {
                            task_id: self.task.id.clone(),
                            scope_name: self.task.scope.name.clone(),
                            iteration: self.iteration,
                            action_type: analysis.next_action.action_type.clone(),
                            element_index: analysis.next_action.element_index,
                            value: analysis.next_action.value.clone(),
                            success: result.success,
                            duration_ms,
                            reason: analysis.next_action.reason.clone(),
                        });
                    }
                }
                Err(e) => {
                    warn!("WorkerAgent [{}]: Action failed: {}", self.task.id, e);
                    self.record_action(&action_desc, false);

                    if let Some(emitter) = &self.message_emitter {
                        let duration_ms = Some(action_start.elapsed().as_millis() as u64);
                        emitter.emit_worker_action(&WorkerActionInfo {
                            task_id: self.task.id.clone(),
                            scope_name: self.task.scope.name.clone(),
                            iteration: self.iteration,
                            action_type: analysis.next_action.action_type.clone(),
                            element_index: analysis.next_action.element_index,
                            value: analysis.next_action.value.clone(),
                            success: false,
                            duration_ms,
                            reason: analysis.next_action.reason.clone(),
                        });
                    }
                }
            }

            // Emit live worker progress for UI (pages/apis/elements) during execution
            if let Some(emitter) = &self.message_emitter {
                emitter.emit_worker_progress(&crate::engines::vision_explorer::message_emitter::WorkerProgressInfo {
                    task_id: self.task.id.clone(),
                    scope_name: self.task.scope.name.clone(),
                    status: "running".to_string(),
                    pages_visited: self.local_visited.len(),
                    apis_discovered: self.discovered_apis.len(),
                    elements_interacted: self.elements_interacted,
                    iterations_used: self.iteration,
                    progress: (analysis.exploration_progress * 100.0).clamp(0.0, 100.0),
                    completion_reason: None,
                });
            }

            // Wait for page to settle
            tokio::time::sleep(tokio::time::Duration::from_millis(300)).await;
        }

        self.build_result(&completion_reason)
    }

    /// Check if URL is within assigned scope
    fn is_in_scope(&self, url: &str) -> bool {
        let normalized = normalize_url(url);

        for pattern in &self.task.scope.url_patterns {
            let full_pattern = self.build_full_url(pattern);
            if normalized.starts_with(&full_pattern) || normalized.contains(pattern) {
                return true;
            }
        }

        // Also check path-only patterns
        if let Ok(parsed) = url::Url::parse(url) {
            let path = parsed.path();
            for pattern in &self.task.scope.url_patterns {
                if path.starts_with(pattern) || path.contains(pattern) {
                    return true;
                }
            }
        }

        false
    }

    /// Partition elements into in-scope and out-of-scope
    fn partition_elements(
        &self,
        elements: &[AnnotatedElement],
    ) -> (Vec<AnnotatedElement>, Vec<AnnotatedElement>) {
        let mut in_scope = Vec::new();
        let mut out_scope = Vec::new();

        for el in elements {
            let href = el.attributes.get("href").map(|s| s.as_str()).unwrap_or("");

            if href.is_empty() || href == "#" || href.starts_with("javascript:") {
                // Non-navigation elements are in-scope
                in_scope.push(el.clone());
            } else if self.is_in_scope(href) || self.is_in_scope(&self.build_full_url(href)) {
                in_scope.push(el.clone());
            } else {
                out_scope.push(el.clone());
            }
        }

        (in_scope, out_scope)
    }

    /// Record cross-scope link discovery
    /// Enhanced: Also reports potential new scopes to GlobalState for dynamic discovery
    fn record_cross_scope(&mut self, url: &str) {
        let full_url = self.build_full_url(url);

        // Avoid duplicates
        if self
            .cross_scope_links
            .iter()
            .any(|l| l.target_url == full_url)
        {
            return;
        }

        // Try to infer target scope from URL
        let target_scope = infer_scope_from_url(&full_url);

        self.cross_scope_links.push(CrossScopeLink {
            source_url: self
                .local_visited
                .iter()
                .last()
                .cloned()
                .unwrap_or_default(),
            target_url: full_url.clone(),
            target_scope: target_scope.clone(),
        });

        // === Dynamic Scope Discovery ===
        // If we discovered a link to a different URL path segment,
        // report it as a potential new scope to GlobalState
        if let Some(scope_id) = &target_scope {
            // Only report if it looks like a major section (not just a specific page)
            if let Ok(parsed) = url::Url::parse(&full_url) {
                let path_segments: Vec<_> = parsed.path()
                    .split('/')
                    .filter(|s| !s.is_empty())
                    .collect();
                
                // If URL has 1-2 path segments, it's likely a major section
                if path_segments.len() <= 2 && !path_segments.is_empty() {
                    let scope_name = path_segments[0].to_string();
                    let entry_path = format!("/{}", path_segments.join("/"));
                    
                    // Record this as a discovered navigation pattern
                    let global_state = self.global_state.clone();
                    let scope_id = scope_id.clone();
                    let full_url = full_url.clone();
                    
                    // Spawn async task to report to global state
                    tokio::spawn(async move {
                        // Record the pattern first
                        if global_state.record_nav_pattern(&entry_path).await {
                            // This is a new pattern, report as potential scope
                            let _ = global_state.report_discovered_url_scope(
                                &scope_id,
                                &scope_name,
                                &full_url,
                                vec![entry_path.clone(), format!("{}/", entry_path)],
                                100, // Low priority for dynamically discovered scopes
                            ).await;
                        }
                    });
                }
            }
        }
    }

    /// Try to expand hidden navigation menus (hamburger buttons, sidebar toggles)
    /// This is a pre-exploration step to reveal collapsed navigation in dashboard UIs
    async fn try_expand_hidden_navigation(&mut self) -> Result<()> {
        info!(
            "WorkerAgent [{}]: Checking for hidden navigation triggers",
            self.task.id
        );

        // Capture current page state
        let page_state = self.browser_tools.capture_page_state_text_mode().await?;
        
        // Find potential hamburger menu / navigation toggle candidates
        let mut hamburger_candidates: Vec<&AnnotatedElement> = page_state
            .annotated_elements
            .iter()
            .filter(|e| {
                // Must be interactive
                let el_type = e.element_type.to_lowercase();
                if !(el_type == "button" || el_type == "clickable" || el_type == "link") {
                    return false;
                }

                // Position check: hamburger menus can be in various positions
                let x = e.bounding_box.x;
                let y = e.bounding_box.y;
                // Left edge (covers left-top, left-middle): x < 150
                // Top bar (covers any position in header): y < 80
                let is_left_edge = x < 150.0;
                let is_top_bar = y < 80.0;
                let is_nav_position = is_left_edge || is_top_bar;

                // Text pattern check: hamburger symbols or empty (icon-only button)
                let text = e.text.trim();
                let is_hamburger_symbol = text == "☰" || text == "≡" || text == "⋮" || text == "⋯" || text.is_empty();

                // === Exclusion check: skip elements that are clearly NOT hamburger menus ===
                let class_str = e.attributes.get("class").map(|s| s.to_lowercase()).unwrap_or_default();
                let id_str = e.attributes.get("id").map(|s| s.to_lowercase()).unwrap_or_default();
                let href_str = e.attributes.get("href").map(|s| s.as_str()).unwrap_or("");
                
                // Exclude logo links (common patterns)
                let is_logo = class_str.contains("logo") 
                    || id_str.contains("logo")
                    || (href_str == "/" && is_hamburger_symbol)  // Empty link to home is usually logo
                    || (href_str.is_empty() && class_str.contains("brand"));
                
                if is_logo {
                    return false;
                }

                // Class pattern check for trigger elements
                let has_trigger_class = e.attributes.get("class")
                    .map(|c| {
                        let cl = c.to_lowercase();
                        cl.contains("hamburger")
                            || cl.contains("toggle")
                            || cl.contains("menu-btn")
                            || cl.contains("menu-button")
                            || cl.contains("nav-trigger")
                            || cl.contains("sidebar-toggle")
                            || cl.contains("drawer-toggle")
                            || cl.contains("collapse")
                            || cl.contains("expand")
                            || cl.contains("burger")
                            || cl.contains("sider")  // Common in Ant Design: ant-layout-sider-trigger
                    })
                    .unwrap_or(false);

                // ARIA pattern check
                let has_aria_popup = e.attributes.get("aria-haspopup").is_some();
                let is_collapsed = e.attributes.get("aria-expanded")
                    .map(|v| v == "false")
                    .unwrap_or(false);
                let controls_nav = e.attributes.get("aria-controls")
                    .map(|v| {
                        let vl = v.to_lowercase();
                        vl.contains("nav") || vl.contains("menu") || vl.contains("sidebar") || vl.contains("drawer")
                    })
                    .unwrap_or(false);

                // ID pattern check
                let has_trigger_id = e.attributes.get("id")
                    .map(|id| {
                        let idl = id.to_lowercase();
                        idl.contains("hamburger")
                            || idl.contains("menu-toggle")
                            || idl.contains("nav-toggle")
                            || idl.contains("sidebar-toggle")
                            || idl.contains("sider")
                    })
                    .unwrap_or(false);

                // Combine conditions:
                // - Navigation position (left edge OR top bar) with empty/symbol text
                // - OR has explicit trigger class/id (position-independent)
                // - OR has ARIA attributes indicating expandable menu
                (is_nav_position && is_hamburger_symbol)
                    || has_trigger_class
                    || has_trigger_id
                    || (has_aria_popup && is_nav_position)
                    || (is_collapsed && controls_nav)
                    || is_collapsed  // Any collapsed element is worth trying
            })
            .collect();

        // Sort by position: prefer top-left corner elements
        hamburger_candidates.sort_by(|a, b| {
            let a_score = a.bounding_box.x + a.bounding_box.y;
            let b_score = b.bounding_box.x + b.bounding_box.y;
            a_score.partial_cmp(&b_score).unwrap()
        });

        // Try clicking up to 2 candidates
        let mut clicked_any = false;
        for el in hamburger_candidates.iter().take(2) {
            info!(
                "WorkerAgent [{}]: Clicking potential hamburger/toggle at [{}, {}] text='{}' class='{}'",
                self.task.id,
                el.bounding_box.x,
                el.bounding_box.y,
                el.text.trim(),
                el.attributes.get("class").map(|s| s.as_str()).unwrap_or("")
            );

            // Click the element
            let click_action = BrowserAction::ClickByIndex { index: el.index };
            if self.browser_tools.execute_action(&click_action).await.is_ok() {
                clicked_any = true;
                // Wait for animation
                tokio::time::sleep(tokio::time::Duration::from_millis(400)).await;

                // Check if new elements appeared (sidebar expanded)
                let new_state = self.browser_tools.capture_page_state_text_mode().await?;
                let element_diff = new_state.annotated_elements.len() as i32 
                    - page_state.annotated_elements.len() as i32;

                if element_diff > 3 {
                    info!(
                        "WorkerAgent [{}]: Navigation expanded! {} new elements detected",
                        self.task.id, element_diff
                    );
                    // Success - navigation expanded
                    return Ok(());
                }
            }
        }

        if clicked_any {
            info!(
                "WorkerAgent [{}]: Clicked potential triggers but no significant nav expansion detected",
                self.task.id
            );
        } else {
            info!(
                "WorkerAgent [{}]: No hamburger/toggle candidates found",
                self.task.id
            );
        }

        Ok(())
    }

    /// Build system prompt (fixed rules / schema, scoped per task)
    fn build_system_prompt(&self) -> String {
        WORKER_SYSTEM_PROMPT_TEMPLATE
            .replace("{scope_name}", &self.task.scope.name)
            .replace("{task_context}", &self.task.context)
            .replace("{entry_url}", &self.task.scope.entry_url)
            .replace("{url_patterns}", &format!("{:?}", self.task.scope.url_patterns))
            .replace("{max_depth}", &self.task.scope.max_depth.to_string())
    }

    /// Build user prompt for LLM (dynamic page state)
    async fn build_user_prompt(
        &self,
        page_state: &PageState,
        in_scope_elements: &[AnnotatedElement],
    ) -> Result<String> {
        // Apply intelligent element filtering
        let filtered = self.element_filter.filter(in_scope_elements);
        let elements_str = format_filtered_for_prompt(&filtered);

        let action_history_str = self
            .action_history
            .iter()
            .rev()
            .take(5)
            .cloned()
            .collect::<Vec<_>>()
            .join("\n");

        let page_summary = if !self.enable_multimodal {
            page_state.visible_text_summary.clone().unwrap_or_default()
        } else {
            String::new()
        };

        // Build navigation path display
        let nav_path = if self.navigation_path.is_empty() {
            "[Entry]".to_string()
        } else {
            self.navigation_path
                .iter()
                .take(5)
                .map(|u| {
                    // Extract last path segment for brevity
                    url::Url::parse(u)
                        .ok()
                        .map(|parsed| parsed.path().split('/').last().unwrap_or("/").to_string())
                        .unwrap_or_else(|| truncate(u, 20))
                })
                .collect::<Vec<_>>()
                .join(" > ")
        };

        // Build scroll status
        let scroll_status = if self.stuck_scroll_count >= 2 {
            "⚠️ REACHED BOTTOM (no new content after scrolling)".to_string()
        } else {
            "Can scroll for more content".to_string()
        };

        let mut credential_context = String::new();
        if let Some(tm) = &self.takeover_manager {
            let tm_read = tm.read().await;
            if let Some(creds) = tm_read.get_credentials_for_llm() {
                credential_context = format!("\n\n## Credentials Available\nUse these credentials if you encounter a login form:\n{}", creds);
            }
            if let Some(user_notes) = tm_read.get_session().user_messages.last() {
                credential_context.push_str(&format!("\n\n## User Note\n{}", user_notes));
            }
        }

        let prompt = WORKER_USER_PROMPT_TEMPLATE
            .replace("{iteration}", &self.iteration.to_string())
            .replace("{max_iterations}", &self.task.max_iterations.to_string())
            .replace("{pages_visited}", &self.local_visited.len().to_string())
            .replace("{apis_discovered}", &self.discovered_apis.len().to_string())
            .replace("{navigation_path}", &nav_path)
            .replace("{scroll_status}", &scroll_status)
            .replace(
                "{action_history}",
                if action_history_str.is_empty() {
                    "None yet"
                } else {
                    &action_history_str
                },
            )
            .replace("{current_url}", &page_state.url)
            .replace("{current_title}", &page_state.title)
            .replace("{page_summary}", &page_summary)
            .replace("{elements}", &elements_str)
            + credential_context.as_str();

        Ok(prompt)
    }

    /// Format elements for prompt (legacy, now using filter)
    fn format_elements(&self, elements: &[AnnotatedElement]) -> String {
        let mut lines = Vec::new();

        for el in elements.iter().take(60) {
            let href = el.attributes.get("href").map(|s| s.as_str()).unwrap_or("");
            let text = truncate(&el.text, 40);
            lines.push(format!(
                "[{}] {} <{}> href=\"{}\" \"{}\"",
                el.index,
                el.element_type,
                el.tag_name,
                truncate(href, 50),
                text
            ));
        }

        if elements.len() > 60 {
            lines.push(format!("... and {} more elements", elements.len() - 60));
        }

        lines.join("\n")
    }

    /// Call LLM with optional screenshot
    async fn call_llm(
        &self,
        system_prompt: &str,
        user_prompt: &str,
        screenshot: Option<&str>,
    ) -> Result<String> {
        let llm_client = LlmClient::new(self.llm_config.clone());

        if self.enable_multimodal && screenshot.is_some() {
            let image = screenshot.map(|s| ImageAttachment::new(s, "png"));
            llm_client
                .completion_with_image(Some(system_prompt), user_prompt, image.as_ref())
                .await
        } else {
            llm_client.completion(Some(system_prompt), user_prompt).await
        }
    }

    /// Build action from analysis
    fn build_action(
        &self,
        analysis: &VlmAnalysisResult,
        elements: &[AnnotatedElement],
    ) -> Result<BrowserAction> {
        let action_type = &analysis.next_action.action_type;

        match action_type.as_str() {
            "click_by_index" => {
                let index = analysis
                    .next_action
                    .element_index
                    .ok_or_else(|| anyhow!("click_by_index requires element_index"))?;

                // Verify element exists and is in scope
                if !elements.iter().any(|e| e.index == index) {
                    return Err(anyhow!("Element index {} not found in scope", index));
                }

                Ok(BrowserAction::ClickByIndex { index })
            }
            "fill_by_index" => {
                let index = analysis
                    .next_action
                    .element_index
                    .ok_or_else(|| anyhow!("fill_by_index requires element_index"))?;
                let value = analysis
                    .next_action
                    .value
                    .clone()
                    .unwrap_or_else(|| "test".to_string());

                Ok(BrowserAction::FillByIndex { index, value })
            }
            "scroll" => {
                // Support direction from value field
                let direction = analysis
                    .next_action
                    .value
                    .as_ref()
                    .map(|v| match v.to_lowercase().as_str() {
                        "up" => crate::engines::vision_explorer::types::ScrollDirection::Up,
                        "left" => crate::engines::vision_explorer::types::ScrollDirection::Left,
                        "right" => crate::engines::vision_explorer::types::ScrollDirection::Right,
                        _ => crate::engines::vision_explorer::types::ScrollDirection::Down,
                    })
                    .unwrap_or(crate::engines::vision_explorer::types::ScrollDirection::Down);

                Ok(BrowserAction::Scroll {
                    coordinates: None,
                    direction,
                    scroll_count: 1,
                })
            }
            "go_back" => {
                // Navigate back using browser history
                Ok(BrowserAction::TypeKeys {
                    keys: vec!["Alt+ArrowLeft".to_string()],
                })
            }
            "press_escape" => {
                // Close modal/dropdown by pressing Escape
                Ok(BrowserAction::TypeKeys {
                    keys: vec!["Escape".to_string()],
                })
            }
            "hover_by_index" => {
                // Hover over an element to reveal hidden menus/dropdowns
                let index = analysis
                    .next_action
                    .element_index
                    .ok_or_else(|| anyhow!("hover_by_index requires element_index"))?;

                // Verify element exists
                if !elements.iter().any(|e| e.index == index) {
                    return Err(anyhow!("Element index {} not found in scope for hover", index));
                }

                Ok(BrowserAction::HoverByIndex { index })
            }
            "screenshot" | "get_elements" => {
                // No-op, will capture on next iteration
                Ok(BrowserAction::Wait { duration_ms: 100 })
            }
            "set_status" => {
                // Treat as completion signal
                Ok(BrowserAction::Wait { duration_ms: 100 })
            }
            _ => {
                warn!("WorkerAgent: Unknown action type: {}", action_type);
                Ok(BrowserAction::Wait { duration_ms: 500 })
            }
        }
    }

    /// Record action in history
    fn record_action(&mut self, action: &str, success: bool) {
        let status = if success { "✓" } else { "✗" };
        let entry = format!("{}. {} {}", self.iteration, status, truncate(action, 60));
        self.action_history.push(entry);
    }

    /// Build full URL from relative path
    fn build_full_url(&self, path: &str) -> String {
        if path.starts_with("http://") || path.starts_with("https://") {
            return path.to_string();
        }

        // Get base from entry URL or first pattern
        let base = if self.task.scope.entry_url.starts_with("http") {
            &self.task.scope.entry_url
        } else {
            return path.to_string();
        };

        if let Ok(base_url) = url::Url::parse(base) {
            if let Ok(full) = base_url.join(path) {
                return full.to_string();
            }
        }

        format!("{}{}", base.trim_end_matches('/'), path)
    }

    /// Build worker result
    fn build_result(&self, completion_reason: &str) -> Result<WorkerResult> {
        self.build_result_with_replan(completion_reason, false)
    }

    fn build_result_with_replan(&self, completion_reason: &str, needs_replan: bool) -> Result<WorkerResult> {
        let duration_ms = self
            .start_time
            .map(|t| t.elapsed().as_millis() as u64)
            .unwrap_or(0);

        // Submit cross-scope links to global state
        for link in &self.cross_scope_links {
            let link_clone = link.clone();
            let global_state = self.global_state.clone();
            tokio::spawn(async move {
                global_state.add_cross_scope_link(link_clone).await;
            });
        }

        Ok(WorkerResult {
            task_id: self.task.id.clone(),
            scope_id: self.task.scope.id.clone(),
            visited_urls: self.local_visited.iter().cloned().collect(),
            discovered_apis: self.discovered_apis.clone(),
            cross_scope_links: self.cross_scope_links.clone(),
            stats: WorkerStats {
                pages_visited: self.local_visited.len(),
                elements_interacted: self.elements_interacted,
                apis_discovered: self.discovered_apis.len(),
                iterations_used: self.iteration,
                duration_ms,
            },
            completion_reason: completion_reason.to_string(),
            needs_replan,
        })
    }

    /// Sync auth state from global -> local browser
    async fn sync_auth_state(&mut self) -> Result<()> {
        if let Some(global_auth) = self.global_state.get_auth_state().await {
            // Apply only if newer than what we last saw/captured
            if global_auth.timestamp > self.last_auth_timestamp {
                debug!("WorkerAgent [{}]: Syncing global auth state (ts: {})", self.task.id, global_auth.timestamp);
                self.browser_tools.apply_auth_state(&global_auth).await?;
                self.last_auth_timestamp = global_auth.timestamp;
            }
        }
        Ok(())
    }

    /// Capture local browser auth -> global
    async fn capture_and_broadcast_auth(&mut self) -> Result<()> {
        debug!("WorkerAgent [{}]: Capturing auth state to broadcast", self.task.id);
        match self.browser_tools.capture_auth_state().await {
            Ok(snapshot) => {
                self.last_auth_timestamp = snapshot.timestamp;
                self.global_state.save_auth_state(snapshot).await;
            }
            Err(e) => {
                warn!("WorkerAgent [{}]: Failed to capture auth state: {}", self.task.id, e);
            }
        }
        Ok(())
    }

    /// Execute login handover sequence (wait for manual user login)
    async fn execute_login_handover(&mut self) -> Result<WorkerResult> {
        info!(
            "WorkerAgent [{}]: pausing for manual user login",
            self.task.id
        );

        // Capture initial page state to detect login fields
        let initial_page_state = self.browser_tools.capture_page_state_text_mode().await.ok();
        
        // Extract login fields from page for frontend display
        let login_fields: Option<Vec<LoginField>> = initial_page_state
            .as_ref()
            .and_then(|ps| login_detector::detect_login_page(ps));

        // Emit takeover request to frontend so user can see the login form
        if let Some(emitter) = &self.message_emitter {
            info!(
                "WorkerAgent [{}]: Emitting takeover request to frontend (fields={})",
                self.task.id,
                login_fields.as_ref().map(|f| f.len()).unwrap_or(0)
            );
            emitter.emit_takeover_request(
                self.iteration,
                "login",
                "Login page detected. Please enter credentials, login manually in browser, or skip.",
                login_fields.as_ref(),
            );
        }

        let max_checks = 180; // 180 * 2s = 6 minutes timeout
        let mut checks = 0;

        // Give user some time to start typing or for page to settle
        tokio::time::sleep(tokio::time::Duration::from_secs(5)).await;

        // Baseline signals (URL/title/auth snapshot) for SPA-style logins where URL may not change
        let mut baseline_url: Option<String> = None;
        let mut baseline_title: Option<String> = None;
        let mut baseline_auth: Option<crate::engines::vision_explorer::multi_agent::AuthSnapshot> =
            None;

        // Best-effort: capture baseline auth snapshot once (do NOT fail login wait if this errors)
        match self.browser_tools.capture_auth_state().await {
            Ok(s) => {
                baseline_auth = Some(s);
                debug!(
                    "WorkerAgent [{}]: Baseline auth captured (cookies={}, local_items={}, session_items={})",
                    self.task.id,
                    baseline_auth.as_ref().map(|a| a.cookies.len()).unwrap_or(0),
                    baseline_auth.as_ref().map(|a| a.local_storage.len()).unwrap_or(0),
                    baseline_auth.as_ref().map(|a| a.session_storage.len()).unwrap_or(0)
                );
            }
            Err(e) => {
                debug!(
                    "WorkerAgent [{}]: Baseline auth capture failed (will rely on page heuristics): {}",
                    self.task.id, e
                );
            }
        }

        loop {
            if checks >= max_checks {
                warn!("WorkerAgent [{}]: Manual login timeout", self.task.id);
                return self.build_result("Manual login timeout");
            }

            // Check TakeoverManager status - user may have clicked "skip" or "already logged in"
            if let Some(tm) = &self.takeover_manager {
                let tm_guard = tm.read().await;
                
                // Check if user skipped login
                if tm_guard.is_login_skipped() {
                    info!(
                        "WorkerAgent [{}]: Login skipped by user, continuing without auth",
                        self.task.id
                    );
                    return self.build_result("Login skipped by user");
                }
                
                // Check if user has credentials ready for AI login
                if tm_guard.has_credentials() {
                    info!(
                        "WorkerAgent [{}]: Credentials received from user, AI will perform login",
                        self.task.id
                    );
                    // Return to main loop to let AI perform login with credentials
                    return self.build_result("Credentials received - AI login pending");
                }
                
                // Check if control has been returned (manual login completed)
                let status = tm_guard.get_status();
                if matches!(status, TakeoverStatus::Returned) && !tm_guard.is_login_detected() {
                    info!(
                        "WorkerAgent [{}]: Manual login completed by user",
                        self.task.id
                    );
                    // Wait a bit for redirects to settle
                    tokio::time::sleep(tokio::time::Duration::from_millis(2000)).await;
                    // Must trigger replan to discover post-login navigation
                    return self.build_result_with_replan("Manual login completed - replan needed", true);
                }
            }

            // Check every 2 seconds
            tokio::time::sleep(tokio::time::Duration::from_secs(2)).await;

            // Capture state (text mode is faster)
            let page_state = match self.browser_tools.capture_page_state_text_mode().await {
                Ok(ps) => ps,
                Err(e) => {
                    warn!(
                        "WorkerAgent: Failed to capture state during login wait: {}",
                        e
                    );
                    continue;
                }
            };

            // Initialize baseline URL/title from first successful capture
            if baseline_url.is_none() {
                baseline_url = Some(page_state.url.clone());
            }
            if baseline_title.is_none() {
                baseline_title = Some(page_state.title.clone());
            }

            self.global_state.mark_visited(&page_state.url).await;

            // Check for login success indicators
            // We use existing detector logic which looks for "Logout", "Dashboard", etc.
            let has_indicators = login_detector::has_logged_in_indicators(&page_state);
            let on_login_route = login_detector::is_login_like_route(&page_state.url);
            let login_form_still_present = login_detector::detect_login_page(&page_state).is_some();

            // SPA-style success signal: auth state changes (cookies/storage) after user action
            let mut auth_changed = false;
            if let Some(base) = baseline_auth.as_ref() {
                if let Ok(now) = self.browser_tools.capture_auth_state().await {
                    auth_changed = auth_snapshot_suggests_logged_in(base, &now);
                    if auth_changed {
                        debug!(
                            "WorkerAgent [{}]: Auth change detected during login wait (cookies={}, local_items={}, session_items={})",
                            self.task.id,
                            now.cookies.len(),
                            now.local_storage.len(),
                            now.session_storage.len()
                        );
                    }
                }
            }

            // Check for invalid/error states
            let lower_url = page_state.url.to_lowercase();
            let is_invalid_page = lower_url == "about:blank"
                || lower_url.starts_with("about:")
                || lower_url.starts_with("chrome:")
                || lower_url.starts_with("chrome-error:")
                || lower_url.is_empty();

            if is_invalid_page {
                warn!(
                    "WorkerAgent [{}]: Invalid page state (url={}), continuing to wait",
                    self.task.id, page_state.url
                );
                checks += 1;
                continue;
            }

            // Check for negative indicators (e.g. reset password page is NOT a login success)
            let is_reset_page = lower_url.contains("reset")
                || lower_url.contains("forgot")
                || lower_url.contains("pwd")
                || lower_url.contains("register");

            // Logic: If we found indicators OR (we are definitely NOT on a login route and URL changed)
            // Note: simply leaving login route is a strong signal of success (or giving up),
            // but usually implies success if combined with continued activity.
            let baseline_url_str = baseline_url.as_deref().unwrap_or("");
            let baseline_title_str = baseline_title.as_deref().unwrap_or("");
            let url_changed = !baseline_url_str.is_empty() && page_state.url != baseline_url_str;
            let title_changed =
                !baseline_title_str.is_empty() && page_state.title != baseline_title_str;

            // Extra heuristic: still on a login-like route, but login form disappeared + auth changed
            // This covers SPA/SSO flows where URL stays the same but user is authenticated.
            let spa_login_success = on_login_route && !login_form_still_present && auth_changed;

            if !is_reset_page
                && (has_indicators
                    || (!on_login_route && url_changed)
                    || spa_login_success
                    || (auth_changed && (title_changed || url_changed) && !login_form_still_present))
            {
                info!(
                    "WorkerAgent [{}]: Login detected! (indicators={}, auth_changed={}, login_form_present={}, url={}). Resuming workflow.",
                    self.task.id, has_indicators, auth_changed, login_form_still_present, page_state.url
                );
                // Wait a bit more for redirects to settle
                tokio::time::sleep(tokio::time::Duration::from_millis(2000)).await;
                // Request re-planning after login to discover new navigation
                return self.build_result_with_replan("Login successful - replan needed", true);
            }

            checks += 1;
            if checks % 10 == 0 {
                info!(
                    "WorkerAgent [{}]: Waiting for user login... ({}s elapsed)",
                    self.task.id,
                    checks * 2
                );
            }
        }
    }
}

/// Heuristic: decide whether auth snapshot changes strongly suggest a successful login.
/// We intentionally avoid logging/returning any secret values; only structure/keys are compared.
fn auth_snapshot_suggests_logged_in(
    baseline: &crate::engines::vision_explorer::multi_agent::AuthSnapshot,
    current: &crate::engines::vision_explorer::multi_agent::AuthSnapshot,
) -> bool {
    // 1) Cookies: new cookie names or auth-like cookies appearing
    let mut base_cookies: std::collections::HashMap<String, String> = std::collections::HashMap::new();
    for c in &baseline.cookies {
        let name = c.get("name").and_then(|v| v.as_str()).unwrap_or("").to_string();
        if name.is_empty() {
            continue;
        }
        let value = c.get("value").and_then(|v| v.as_str()).unwrap_or("").to_string();
        base_cookies.insert(name, value);
    }
    let mut cur_cookies: std::collections::HashMap<String, String> = std::collections::HashMap::new();
    for c in &current.cookies {
        let name = c.get("name").and_then(|v| v.as_str()).unwrap_or("").to_string();
        if name.is_empty() {
            continue;
        }
        let value = c.get("value").and_then(|v| v.as_str()).unwrap_or("").to_string();
        cur_cookies.insert(name, value);
    }

    let auth_cookie_names = ["token", "jwt", "session", "sid", "sso", "auth"];
    let has_new_cookie_name = cur_cookies.keys().any(|k| !base_cookies.contains_key(k));
    let has_auth_cookie = cur_cookies.keys().any(|k| {
        let lk = k.to_lowercase();
        auth_cookie_names.iter().any(|p| lk.contains(p))
    });
    let has_auth_cookie_changed = cur_cookies.iter().any(|(k, v)| {
        let lk = k.to_lowercase();
        if !auth_cookie_names.iter().any(|p| lk.contains(p)) {
            return false;
        }
        match base_cookies.get(k) {
            Some(prev) => prev != v && !v.is_empty(),
            None => !v.is_empty(),
        }
    });

    // 2) Storage: new/changed auth-like keys
    let auth_key_patterns = ["token", "auth", "session", "jwt", "sid", "user", "access", "refresh"];
    let looks_like_auth_kv = |key: &str, val: &str| {
        let lk = key.to_lowercase();
        if !auth_key_patterns.iter().any(|p| lk.contains(p)) {
            return false;
        }
        // Avoid false positives on empty/default values
        let v = val.trim();
        if v.is_empty() {
            return false;
        }
        // Token-ish heuristics
        v.len() >= 12 || v.contains('.') || v.contains("Bearer ")
    };

    let storage_changed = current
        .local_storage
        .iter()
        .any(|(k, v)| looks_like_auth_kv(k, v) && baseline.local_storage.get(k) != Some(v))
        || current
            .session_storage
            .iter()
            .any(|(k, v)| looks_like_auth_kv(k, v) && baseline.session_storage.get(k) != Some(v));

    // 3) Simple structural change: storage/cookie counts increased notably
    let cookie_count_increase = current.cookies.len().saturating_sub(baseline.cookies.len()) >= 1;
    let local_count_increase =
        current.local_storage.len().saturating_sub(baseline.local_storage.len()) >= 1;

    (has_auth_cookie && (has_auth_cookie_changed || storage_changed))
        || (storage_changed && (cookie_count_increase || local_count_increase))
        || (has_new_cookie_name && has_auth_cookie)
}

/// Normalize URL for comparison
fn normalize_url(url: &str) -> String {
    let mut normalized = url.trim().to_string();

    // Remove fragment
    if let Some(idx) = normalized.find('#') {
        normalized.truncate(idx);
    }

    // Remove trailing slash
    if normalized.len() > 1 && normalized.ends_with('/') {
        normalized.pop();
    }

    normalized
}

/// Truncate string with ellipsis
fn truncate(s: &str, max_len: usize) -> String {
    if s.len() <= max_len {
        s.to_string()
    } else {
        format!("{}...", &s[..max_len.saturating_sub(3)])
    }
}

/// Parse API string like "GET /api/users" into DiscoveredApi
fn parse_api_string(s: &str) -> Option<DiscoveredApi> {
    let parts: Vec<&str> = s.trim().splitn(2, ' ').collect();
    if parts.len() < 2 {
        return None;
    }

    let method = parts[0].to_uppercase();
    let path = parts[1].to_string();

    // Validate method
    if !["GET", "POST", "PUT", "DELETE", "PATCH", "HEAD", "OPTIONS"].contains(&method.as_str()) {
        return None;
    }

    Some(DiscoveredApi {
        method,
        path: path.clone(),
        full_url: path,
        parameters: Default::default(),
        status_code: None,
    })
}

/// Infer scope from URL path
fn infer_scope_from_url(url: &str) -> Option<String> {
    let parsed = url::Url::parse(url).ok()?;
    let path = parsed.path();

    // Get first path segment as scope
    let segments: Vec<&str> = path.split('/').filter(|s| !s.is_empty()).collect();
    segments.first().map(|s| s.to_string())
}
