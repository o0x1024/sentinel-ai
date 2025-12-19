//! Manager Agent for multi-agent exploration
//!
//! Responsibilities:
//! - Analyze homepage navigation structure
//! - Identify exploration scopes (URL patterns, modules)
//! - Assign tasks to Worker agents
//! - Coordinate cross-scope discoveries

use super::element_filter::{format_filtered_for_prompt, ElementFilter, ElementFilterConfig};
use super::global_state::GlobalExplorerState;
use super::types::*;
use crate::engines::vision_explorer::tools::BrowserTools;
use crate::engines::vision_explorer::types::{
    AnnotatedElement, BrowserAction, PageState, ViewportSize,
};
use crate::engines::{LlmClient, LlmConfig};
use anyhow::{anyhow, Result};
use serde_json::Value;
use std::sync::Arc;
use tokio::sync::RwLock;
use tracing::{debug, info, warn};

/// System prompt for navigation analysis (fixed format and instructions)
const NAV_ANALYSIS_SYSTEM_PROMPT: &str = r#"You are analyzing a website's homepage navigation structure to divide it into exploration scopes.

## Task
Analyze the provided page elements and spatial layout to identify:
1. Main navigation entries (top nav, primary menu) - Note: these might be <div> or <span> elements with text if they are clickable.
2. Secondary navigation (sidebar, footer links)
3. Login/authentication entry points
4. Logical scope divisions based on URL patterns

## Output Format (JSON)
```json
{
  "main_nav": [
    {"text": "Products", "href": "/products", "has_submenu": true}
  ],
  "secondary_nav": [
    {"text": "About", "href": "/about", "has_submenu": false}
  ],
  "auth_entries": [
    {"text": "Login", "href": "/login", "has_submenu": false}
  ],
  "suggested_scopes": [
    {
      "id": "products",
      "name": "Products",
      "url_patterns": ["/products", "/product/", "/catalog"],
      "entry_url": "/products",
      "priority": 1
    }
  ],
  "is_spa": false,
  "has_login_gate": false
}
```

Analyze the navigation and return ONLY valid JSON (no markdown fences)."#;

/// User prompt template for navigation analysis (dynamic content)
const NAV_ANALYSIS_USER_PROMPT: &str = r#"## Page Information
URL: {url}
Title: {title}

## Page Layout & Elements
{elements}"#;

/// System prompt for LLM-driven navigation discovery
const NAV_DISCOVERY_SYSTEM_PROMPT: &str = r#"You are a navigation discovery expert. Your goal is to identify all primary and secondary navigation menus, sidebars, and hidden navigation triggers in a web application.

## Your Task
Analyze the provided elements from the Header and Sidebar regions. Suggest a sequence of interactive elements to click or hover over to:
1. Reveal hidden menus (hamburger menus, dropdowns, profile menus).
2. Enumerate all major functional sections of the application.
3. Identify multi-level navigation structures.

## Critical Instructions
- **Do not blindly click every button.** If an element lacks clear text and its attributes (aria-label, title) suggest it is a common utilitarian tool (search, theme toggle, translate, fullscreen, notifications), ignore it for navigation discovery.
- Focus on elements that likely lead to NEW sections or reveal a SITE-WIDE menu (e.g., hamburger menus, "Menu" text, profile avatars with dropdowns, gear icons for settings).
- If you have already explored all likely navigation candidates in the current view, or if the current view contains no navigation triggers, set `is_complete` to true.

## Output Format (JSON)
Return a list of steps to take. Each step should include the element index and the reason for interaction.
```json
{
  "steps": [
    {
      "index": 12,
      "reason": "Expand 'Settings' dropdown to find sub-pages",
      "expected_outcome": "expand_menu"
    }
  ],
  "is_complete": false
}
```
Return ONLY valid JSON."#;

/// User prompt for discovery
const NAV_DISCOVERY_USER_PROMPT: &str = r#"## Current Context
Base URL: {base_url}
Current URL: {current_url}
Already Discovered Scopes: {discovered_scopes}
Already Interacted Elements (This View): {interacted_elements}

## Region-Filtered Elements (Header & Sidebars only)
{elements}

## Instructions
Based on the visual layout and element descriptions above, suggest the next set of elements to interact with to discover the full site map. If no meaningful navigation is found, set is_complete to true."#;

/// A single step recommended by LLM for discovery
#[derive(Debug, serde::Deserialize)]
struct DiscoveryStep {
    pub index: usize,
    pub reason: String,
    pub expected_outcome: String,
}

/// LLM response for navigation discovery
#[derive(Debug, serde::Deserialize)]
struct DiscoveryResponse {
    pub steps: Vec<DiscoveryStep>,
    pub is_complete: bool,
}

/// Manager Agent for coordinating multi-agent exploration
pub struct ManagerAgent {
    /// Browser tools for page interaction
    browser_tools: Arc<BrowserTools>,
    /// LLM configuration
    llm_config: LlmConfig,
    /// Global shared state
    global_state: Arc<GlobalExplorerState>,
    /// Target URL
    target_url: String,
    /// Multi-agent configuration
    config: MultiAgentConfig,
    /// Navigation analysis result
    nav_analysis: Option<NavigationAnalysis>,
    /// Element filter for context optimization
    element_filter: ElementFilter,
    /// Element manager for stable indexing and deduplication
    element_manager: crate::engines::vision_explorer::element_manager::ElementManager,
}

impl ManagerAgent {
    /// Create new Manager Agent
    pub fn new(
        browser_tools: Arc<BrowserTools>,
        llm_config: LlmConfig,
        global_state: Arc<GlobalExplorerState>,
        target_url: String,
        config: MultiAgentConfig,
    ) -> Self {
        Self {
            browser_tools,
            llm_config,
            global_state,
            target_url,
            config,
            nav_analysis: None,
            element_filter: ElementFilter::new(crate::engines::vision_explorer::multi_agent::element_filter::ElementFilterConfig::default()),
            element_manager: crate::engines::vision_explorer::element_manager::ElementManager::new(),
        }
    }

    /// Analyze homepage and create exploration plan
    pub async fn analyze_and_plan(&mut self) -> Result<Vec<WorkerTask>> {
        self.analyze_and_plan_internal(true).await
    }

    /// Analyze current page without navigation (for re-planning after login)
    pub async fn analyze_current_page(&mut self) -> Result<Vec<WorkerTask>> {
        self.analyze_and_plan_internal(false).await
    }

    /// Internal method for analysis with optional navigation
    async fn analyze_and_plan_internal(&mut self, navigate: bool) -> Result<Vec<WorkerTask>> {
        if navigate {
            info!(
                "ManagerAgent: Starting navigation analysis for {}",
                self.target_url
            );

            // Navigate to homepage
            let nav_action = crate::engines::vision_explorer::types::BrowserAction::Navigate {
                url: self.target_url.clone(),
            };
            self.browser_tools.execute_action(&nav_action).await?;

            // Wait for page to stabilize
            tokio::time::sleep(tokio::time::Duration::from_millis(1000)).await;
        } else {
            info!("ManagerAgent: Analyzing current page (post-login re-planning)");
        }

        // Capture page state (text mode for navigation analysis)
        let page_state = self.browser_tools.capture_page_state_text_mode().await?;

        info!(
            "ManagerAgent: Captured page state - URL: {}, Title: {}, Elements: {}",
            page_state.url,
            page_state.title,
            page_state.annotated_elements.len()
        );

        // Mark page as visited
        self.global_state.mark_visited(&page_state.url).await;

        // Post-login: proactively enumerate top-level menus to create multiple scopes.
        // Many admin dashboards keep sidebar collapsed (icon-only), so a passive element snapshot
        // often yields 0 nav entries/scopes. Here we click/hover likely nav items and infer scopes
        // from the resulting URL path segment (e.g. /asset, /threat, /security).
        if !navigate {
            if let Ok(scopes) = self.discover_scopes_from_top_level_menus(&page_state).await {
                if scopes.len() >= 2 {
                    info!(
                        "ManagerAgent: Discovered {} top-level scopes via menu enumeration (post-login)",
                        scopes.len()
                    );

                    let analysis = NavigationAnalysis {
                        main_nav: vec![],
                        secondary_nav: vec![],
                        auth_entries: vec![],
                        suggested_scopes: scopes,
                        // Treat as SPA by default; many dashboards are SPA
                        is_spa: true,
                        has_login_gate: false,
                    };
                    self.nav_analysis = Some(analysis.clone());

                    // Register scopes
                    for scope in &analysis.suggested_scopes {
                        for pattern in &scope.url_patterns {
                            let full_pattern = self.build_full_url(pattern);
                            self.global_state
                                .register_scope(&full_pattern, &scope.id)
                                .await;
                        }
                    }

                    let tasks = self.create_worker_tasks(&analysis)?;
                    info!(
                        "ManagerAgent: Created {} worker tasks from {} scopes",
                        tasks.len(),
                        analysis.suggested_scopes.len()
                    );
                    return Ok(tasks);
                } else {
                    info!(
                        "ManagerAgent: Menu enumeration found {} scopes, falling back to LLM nav analysis",
                        scopes.len()
                    );
                }
            } else {
                warn!("ManagerAgent: Menu enumeration failed, falling back to LLM nav analysis");
            }
        }

        // Analyze navigation structure
        let analysis = self.analyze_navigation(&page_state).await?;
        self.nav_analysis = Some(analysis.clone());

        // Register scopes
        for scope in &analysis.suggested_scopes {
            for pattern in &scope.url_patterns {
                let full_pattern = self.build_full_url(pattern);
                self.global_state
                    .register_scope(&full_pattern, &scope.id)
                    .await;
            }
        }

        // Create worker tasks
        let tasks = self.create_worker_tasks(&analysis)?;

        info!(
            "ManagerAgent: Created {} worker tasks from {} scopes",
            tasks.len(),
            analysis.suggested_scopes.len()
        );

        Ok(tasks)
    }

    /// Post-login: click/hover likely top-level menu items and infer scopes from URL path segments.
    /// Enhanced version: supports non-standard layouts (right sidebar, bottom nav, hamburger menus)
    /// and recursive menu exploration for multi-level navigation.
    async fn discover_scopes_from_top_level_menus(
        &mut self,
        base_state: &PageState,
    ) -> Result<Vec<ExplorationScope>> {
        let base_url = base_state.url.clone();
        let mut discovered_scopes: Vec<ExplorationScope> = Vec::new();
        let mut visited_fps: std::collections::HashSet<String> = std::collections::HashSet::new();

        info!(
            "ManagerAgent: Starting LLM-driven menu discovery at {}",
            base_url
        );

        // Maximum discovery iterations to prevent infinite loops (budget: 10 calls)
        for iteration in 0..10 {
            // Check for pause/stop
            self.global_state.wait_if_paused().await;

            // 1. Capture current state for analysis
            let current_state = match self.browser_tools.capture_page_state_text_mode().await {
                Ok(s) => s,
                Err(e) => {
                    warn!(
                        "ManagerAgent: Failed to capture state during discovery: {}",
                        e
                    );
                    break;
                }
            };

            // 2. Ask LLM for next discovery steps
            let discovery = match self
                .analyze_discovery_steps(&current_state, &discovered_scopes, &visited_fps)
                .await
            {
                Ok(d) => d,
                Err(e) => {
                    warn!(
                        "ManagerAgent: Discovery analysis failed at iteration {}: {}",
                        iteration, e
                    );
                    break;
                }
            };

            if discovery.is_complete || discovery.steps.is_empty() {
                info!("ManagerAgent: LLM signaled discovery complete or no more steps suggested.");
                break;
            }

            // 3. Execute recommended steps
            let mut actions_taken = 0;
            for step in discovery.steps {
                // Find element for fingerprinting from the state the LLM analyzed
                let Some(el_desc) = current_state
                    .annotated_elements
                    .iter()
                    .find(|e| e.index == step.index as u32)
                else {
                    warn!(
                        "ManagerAgent: LLM suggested unreachable index {}",
                        step.index
                    );
                    continue;
                };

                // Generate fingerprint for stable tracking
                let fp = crate::engines::vision_explorer::element_manager::ElementManager::generate_fingerprint(
                    el_desc,
                    &current_state.url,
                );
                if visited_fps.contains(&fp) {
                    continue;
                }
                visited_fps.insert(fp.clone());
                actions_taken += 1;

                info!(
                    "ManagerAgent: Executing discovery action: {} (reason: {})",
                    step.expected_outcome, step.reason
                );

                // Re-resolve element to current DOM (handling potential drift from previous clicks in this iteration)
                let latest_state = self.browser_tools.capture_page_state_text_mode().await?;
                let resolved_idx = latest_state
                    .annotated_elements
                    .iter()
                    .find(|e| {
                        crate::engines::vision_explorer::element_manager::ElementManager::generate_fingerprint(e, &latest_state.url) == fp
                    })
                    .map(|e| e.index)
                    .unwrap_or(step.index as u32);

                // Perform action (Click is the main explorer for menus)
                let _ = self
                    .browser_tools
                    .execute_action(&BrowserAction::ClickByIndex {
                        index: resolved_idx,
                    })
                    .await;

                // Wait for potential navigation or DOM change
                tokio::time::sleep(tokio::time::Duration::from_millis(500)).await;

                // Capture result
                let after_state = self.browser_tools.capture_page_state_text_mode().await?;

                // Check for URL-based scope discovery
                if after_state.url != current_state.url {
                    if let Some(scope) = self.infer_scope_from_url_change(
                        &base_url,
                        &after_state.url,
                        &after_state.title,
                    ) {
                        if !discovered_scopes.iter().any(|s| s.id == scope.id) {
                            info!(
                                "ManagerAgent: Discovered scope '{}' via LLM-guided click",
                                scope.name
                            );
                            discovered_scopes.push(scope);
                        }
                    }

                    // Reset to base URL to explore other branches, unless it's a sub-menu navigation we want to continue
                    // For now, always reset to base to maintain high-level discovery consistency
                    let _ = self
                        .browser_tools
                        .execute_action(&BrowserAction::Navigate {
                            url: base_url.clone(),
                        })
                        .await;
                    tokio::time::sleep(tokio::time::Duration::from_millis(400)).await;
                    break; // Re-analyze from clean base state
                } else {
                    // No URL change, check if DOM changed significantly (likely a dropdown opened)
                    let dom_diff = (after_state.annotated_elements.len() as i32
                        - latest_state.annotated_elements.len() as i32)
                        .abs();
                    if dom_diff > 3 {
                        debug!("ManagerAgent: DOM changed significantly after interaction, continuing discovery in current view.");
                        // We don't break, so LLM can see the new elements in the next loop iteration (after capturing state)
                        break;
                    }
                }
            }

            if actions_taken == 0 {
                info!("ManagerAgent: No more unique discovery steps to take in current view. Terminating discovery loop.");
                break;
            }
        }

        Ok(discovered_scopes)
    }

    /// Infer an exploration scope from a URL discovery
    fn infer_scope_from_url_change(
        &self,
        base_url: &str,
        new_url: &str,
        page_title: &str,
    ) -> Option<ExplorationScope> {
        let base_path_segs: Vec<String> = url::Url::parse(base_url)
            .ok()?
            .path()
            .split('/')
            .filter(|s| !s.is_empty())
            .map(|s| s.to_string())
            .collect();

        let new_parsed = url::Url::parse(new_url).ok()?;
        let new_path_segs: Vec<String> = new_parsed
            .path()
            .split('/')
            .filter(|s| !s.is_empty())
            .map(|s| s.to_string())
            .collect();

        // Find first differing path segment
        let mut diff_seg = None;
        for i in 0..std::cmp::max(base_path_segs.len(), new_path_segs.len()) {
            let b = base_path_segs.get(i);
            let n = new_path_segs.get(i);
            if n.is_some() && n != b {
                diff_seg = n.cloned();
                break;
            }
        }

        // Fallback to hash if path didn't change
        if diff_seg.is_none() {
            if let Some(frag) = new_parsed.fragment() {
                diff_seg = frag
                    .trim_start_matches('/')
                    .split('/')
                    .next()
                    .map(|s| s.to_string());
            }
        }

        let seg = diff_seg?;
        if seg == "login" || seg == "signin" || seg.is_empty() {
            return None;
        }

        let scope_id = sanitize_id(&seg);
        let name = if page_title.is_empty() {
            seg.clone()
        } else {
            // Use title if it's short, otherwise use segment
            let parts: Vec<_> = page_title.split(&['|', '-', '—'][..]).collect();
            let first = parts[0].trim();
            if first.len() < 20 {
                first.to_string()
            } else {
                seg.clone()
            }
        };

        // Create suggested pattern (segment-based)
        let pattern = format!("/{}", seg);

        Some(ExplorationScope {
            id: scope_id,
            name,
            url_patterns: vec![pattern.clone(), format!("{}/", pattern)],
            entry_url: new_url.to_string(),
            max_depth: 3,
            priority: 50,
        })
    }

    /// Analyze navigation structure from page state
    async fn analyze_navigation(&self, page_state: &PageState) -> Result<NavigationAnalysis> {
        // Format elements for LLM - using the smarter filter from worker
        let filtered = self.element_filter.filter(&page_state.annotated_elements);
        let elements_str =
            self.format_elements_for_analysis(&filtered.elements, page_state.viewport.as_ref());

        // Build user prompt with dynamic content
        let user_prompt = NAV_ANALYSIS_USER_PROMPT
            .replace("{url}", &page_state.url)
            .replace("{title}", &page_state.title)
            .replace("{elements}", &elements_str);

        // Call LLM with system prompt for fixed format, user prompt for dynamic content
        let llm_client = LlmClient::new(self.llm_config.clone());
        let response = llm_client
            .completion(Some(NAV_ANALYSIS_SYSTEM_PROMPT), &user_prompt)
            .await?;

        // Parse response
        let mut analysis = self.parse_nav_analysis(&response)?;

        // Filter auth entries to ensuring they are within target domain
        let target_domain = url::Url::parse(&self.target_url)
            .ok()
            .and_then(|u| u.domain().map(|d| d.to_string()))
            .unwrap_or_default();

        if !target_domain.is_empty() {
            analysis.auth_entries.retain(|entry| {
                if entry.href.starts_with("http") {
                    if let Ok(u) = url::Url::parse(&entry.href) {
                        return u
                            .domain()
                            .map(|d| d.ends_with(&target_domain))
                            .unwrap_or(false);
                    }
                }
                true // Relative paths are fine
            });
        }

        info!(
            "ManagerAgent: Found {} main nav, {} secondary nav, {} auth entries, {} scopes",
            analysis.main_nav.len(),
            analysis.secondary_nav.len(),
            analysis.auth_entries.len(),
            analysis.suggested_scopes.len()
        );

        Ok(analysis)
    }

    /// Ask LLM to suggest the next discovery steps (interactive navigation elements)
    async fn analyze_discovery_steps(
        &self,
        page_state: &PageState,
        discovered_scopes: &[ExplorationScope],
        visited_fps: &std::collections::HashSet<String>,
    ) -> Result<DiscoveryResponse> {
        let discovered_str = discovered_scopes
            .iter()
            .map(|s| format!("{} ({})", s.name, s.url_patterns.join(", ")))
            .collect::<Vec<_>>()
            .join("; ");

        // Identify which elements in current page_state have already been visited
        let interacted_str = page_state
            .annotated_elements
            .iter()
            .filter(|e| {
                let fp = crate::engines::vision_explorer::element_manager::ElementManager::generate_fingerprint(e, &page_state.url);
                visited_fps.contains(&fp)
            })
            .map(|e| format!("[{}] \"{}\"", e.index, truncate(&e.text, 30)))
            .collect::<Vec<_>>()
            .join(", ");

        let interacted_final = if interacted_str.is_empty() {
            "None".to_string()
        } else {
            interacted_str
        };

        // Format elements with region filter (Header & Sidebars only)
        let elements_str = self.format_elements_for_analysis(
            &page_state.annotated_elements,
            page_state.viewport.as_ref(),
        );

        let user_prompt = NAV_DISCOVERY_USER_PROMPT
            .replace("{base_url}", &self.target_url)
            .replace("{current_url}", &page_state.url)
            .replace("{discovered_scopes}", &discovered_str)
            .replace("{interacted_elements}", &interacted_final)
            .replace("{elements}", &elements_str);

        let llm_client = LlmClient::new(self.llm_config.clone());
        let response_text = llm_client
            .completion(Some(NAV_DISCOVERY_SYSTEM_PROMPT), &user_prompt)
            .await?;

        // Extract JSON from potential markdown fences
        let json_str = if let Some(start) = response_text.find("```json") {
            let rest = &response_text[start + 7..];
            if let Some(end) = rest.find("```") {
                &rest[..end]
            } else {
                rest
            }
        } else if let Some(start) = response_text.find('{') {
            &response_text[start..]
        } else {
            &response_text
        };

        let discovery: DiscoveryResponse = serde_json::from_str(json_str).map_err(|e| {
            anyhow!(
                "Failed to parse discovery response: {}. Raw: {}",
                e,
                response_text
            )
        })?;

        Ok(discovery)
    }

    /// Format elements for LLM analysis with spatial partitioning
    fn format_elements_for_analysis(
        &self,
        elements: &[AnnotatedElement],
        viewport: Option<&ViewportSize>,
    ) -> String {
        // Filter to navigation-relevant elements first
        let nav_elements: Vec<_> = elements
            .iter()
            .filter(|e| {
                let tag = e.tag_name.to_lowercase();
                let el_type = e.element_type.to_lowercase();

                // Focus on links and navigation elements
                tag == "a"
                    || tag == "nav"
                    || tag == "button"
                    || el_type == "link"
                    || el_type == "button"
                    || e.attributes
                        .get("role")
                        .map(|r| r == "navigation" || r == "menuitem")
                        .unwrap_or(false)
            })
            // .take(100) // Accessing all relevant elements for correct partitioning, limit later if needed
            .cloned()
            .collect();

        // Partition elements into regions
        let regions = partition_elements(nav_elements, viewport);

        let mut output = String::new();
        output.push_str("## SPATIAL NAVIGATION STRUCTURE\n");

        for (region, elements) in regions {
            if elements.is_empty() {
                continue;
            }

            // Manager exploration focus: primarily include Header and Sidebars
            // Include items in MainContent ONLY if they have navigation-related roles/tags/classes
            if matches!(region, SpatialRegion::MainContent | SpatialRegion::Footer) {
                let has_nav = elements.iter().any(|e| {
                    let role = e.attributes.get("role").map(|s| s.as_str()).unwrap_or("");
                    let class = e.attributes.get("class").map(|s| s.as_str()).unwrap_or("");
                    role.contains("nav")
                        || role.contains("menu")
                        || class.contains("nav")
                        || class.contains("menu")
                        || class.contains("item")
                        || e.tag_name.to_lowercase() == "nav"
                });

                if !has_nav {
                    continue;
                }
            }

            output.push_str(&format!(
                "\n[AREA: {:?}] ({} elements)\n",
                region,
                elements.len()
            ));

            // Sort elements by y, then x within region for natural reading order
            let mut sorted_elements = elements;
            sorted_elements.sort_by(|a, b| {
                a.bounding_box
                    .y
                    .partial_cmp(&b.bounding_box.y)
                    .unwrap_or(std::cmp::Ordering::Equal)
                    .then_with(|| {
                        a.bounding_box
                            .x
                            .partial_cmp(&b.bounding_box.x)
                            .unwrap_or(std::cmp::Ordering::Equal)
                    })
            });

            // Limit elements per region to avoid token overflow
            // Header/Sidebar are most important, Main Content less so
            let limit = match region {
                SpatialRegion::TopHeader
                | SpatialRegion::LeftSidebar
                | SpatialRegion::RightSidebar => 50,
                SpatialRegion::MainContent | SpatialRegion::Footer => 20,
            };

            for el in sorted_elements.iter().take(limit) {
                let href = el.attributes.get("href").map(|s| s.as_str()).unwrap_or("");
                let text = truncate(&el.text, 50);

                // Try to find descriptive attributes for icon-only buttons
                let aria_label = el
                    .attributes
                    .get("aria-label")
                    .map(|s| s.as_str())
                    .unwrap_or("");
                let title = el.attributes.get("title").map(|s| s.as_str()).unwrap_or("");
                let desc_parts = [aria_label, title]
                    .into_iter()
                    .filter(|s| !s.is_empty())
                    .collect::<Vec<_>>();
                let attr_desc = if !desc_parts.is_empty() {
                    format!(" attrs=\"{}\"", desc_parts.join(", "))
                } else {
                    String::new()
                };

                // Add visual cues to the string
                let visual_hint = if el
                    .enhanced_attributes
                    .as_ref()
                    .map(|a| a.computed_styles.is_bold)
                    .unwrap_or(false)
                {
                    " [BOLD]"
                } else {
                    ""
                };

                output.push_str(&format!(
                    "- [{}] <{}> \"{}\" (href=\"{}\") pos=({:.0},{:.0}){}{}\n",
                    el.index,
                    el.tag_name,
                    text,
                    href,
                    el.bounding_box.x,
                    el.bounding_box.y,
                    attr_desc,
                    visual_hint
                ));
            }

            if sorted_elements.len() > limit {
                output.push_str(&format!(
                    "... ({} more elements truncated)\n",
                    sorted_elements.len() - limit
                ));
            }
        }

        output
    }

    /// Parse navigation analysis from LLM response
    fn parse_nav_analysis(&self, response: &str) -> Result<NavigationAnalysis> {
        // Extract JSON from response
        let json_str = extract_json(response);

        let json: Value = serde_json::from_str(&json_str)
            .map_err(|e| anyhow!("Failed to parse navigation analysis JSON: {}", e))?;

        // Parse main_nav
        let main_nav = self.parse_nav_entries(json.get("main_nav"));
        let secondary_nav = self.parse_nav_entries(json.get("secondary_nav"));
        let auth_entries = self.parse_nav_entries(json.get("auth_entries"));

        // Parse suggested scopes
        let suggested_scopes = self.parse_scopes(json.get("suggested_scopes"));

        // Parse flags
        let is_spa = json
            .get("is_spa")
            .and_then(|v| v.as_bool())
            .unwrap_or(false);
        let has_login_gate = json
            .get("has_login_gate")
            .and_then(|v| v.as_bool())
            .unwrap_or(false);

        // If no scopes suggested, create default scope
        let suggested_scopes = if suggested_scopes.is_empty() {
            self.create_default_scopes(&main_nav, &secondary_nav)
        } else {
            suggested_scopes
        };

        Ok(NavigationAnalysis {
            main_nav,
            secondary_nav,
            auth_entries,
            suggested_scopes,
            is_spa,
            has_login_gate,
        })
    }

    /// Parse navigation entries from JSON
    fn parse_nav_entries(&self, value: Option<&Value>) -> Vec<NavigationEntry> {
        let Some(array) = value.and_then(|v| v.as_array()) else {
            return Vec::new();
        };

        array
            .iter()
            .filter_map(|v| {
                let text = v.get("text")?.as_str()?.to_string();
                let href = v.get("href")?.as_str()?.to_string();
                let has_submenu = v
                    .get("has_submenu")
                    .and_then(|v| v.as_bool())
                    .unwrap_or(false);
                let element_index = v
                    .get("element_index")
                    .and_then(|v| v.as_u64())
                    .map(|n| n as u32);

                Some(NavigationEntry {
                    text,
                    href,
                    element_index,
                    inferred_scope: None,
                    has_submenu,
                })
            })
            .collect()
    }

    /// Parse exploration scopes from JSON
    fn parse_scopes(&self, value: Option<&Value>) -> Vec<ExplorationScope> {
        let Some(array) = value.and_then(|v| v.as_array()) else {
            return Vec::new();
        };

        array
            .iter()
            .filter_map(|v| {
                let id = v.get("id")?.as_str()?.to_string();
                let name = v.get("name")?.as_str()?.to_string();
                let entry_url = v.get("entry_url")?.as_str()?.to_string();

                let url_patterns: Vec<String> = v
                    .get("url_patterns")
                    .and_then(|p| p.as_array())
                    .map(|arr| {
                        arr.iter()
                            .filter_map(|s| s.as_str().map(String::from))
                            .collect()
                    })
                    .unwrap_or_else(|| vec![entry_url.clone()]);

                let priority = v.get("priority").and_then(|p| p.as_u64()).unwrap_or(1) as u32;

                Some(ExplorationScope {
                    id,
                    name,
                    url_patterns,
                    entry_url,
                    max_depth: self.config.default_max_depth,
                    priority,
                })
            })
            .collect()
    }

    /// Create default scopes from navigation entries
    fn create_default_scopes(
        &self,
        main_nav: &[NavigationEntry],
        secondary_nav: &[NavigationEntry],
    ) -> Vec<ExplorationScope> {
        let mut scopes = Vec::new();
        let mut priority = 1u32;

        // Create scope for each main nav entry
        for entry in main_nav {
            if entry.href.is_empty() || entry.href == "#" || entry.href.starts_with("javascript:") {
                continue;
            }

            let id = sanitize_id(&entry.text);
            scopes.push(ExplorationScope {
                id: id.clone(),
                name: entry.text.clone(),
                url_patterns: vec![entry.href.clone()],
                entry_url: entry.href.clone(),
                max_depth: self.config.default_max_depth,
                priority,
            });
            priority += 1;
        }

        // Add secondary nav with lower priority
        for entry in secondary_nav.iter().take(5) {
            if entry.href.is_empty() || entry.href == "#" {
                continue;
            }

            let id = sanitize_id(&entry.text);
            if scopes.iter().any(|s| s.id == id) {
                continue; // Skip duplicates
            }

            scopes.push(ExplorationScope {
                id,
                name: entry.text.clone(),
                url_patterns: vec![entry.href.clone()],
                entry_url: entry.href.clone(),
                max_depth: self.config.default_max_depth.saturating_sub(1),
                priority: priority + 10, // Lower priority
            });
            priority += 1;
        }

        // If still no scopes, create a single "main" scope
        if scopes.is_empty() {
            scopes.push(ExplorationScope {
                id: "main".to_string(),
                name: "Main".to_string(),
                url_patterns: vec!["/".to_string()],
                entry_url: "/".to_string(),
                max_depth: self.config.default_max_depth,
                priority: 1,
            });
        }

        scopes
    }

    /// Create worker tasks from navigation analysis
    fn create_worker_tasks(&self, analysis: &NavigationAnalysis) -> Result<Vec<WorkerTask>> {
        let mut tasks = Vec::new();

        // Sort scopes by priority
        let mut scopes = analysis.suggested_scopes.clone();
        scopes.sort_by_key(|s| s.priority);

        for scope in scopes {
            // Normalize scope entry_url to a navigable URL (defensive against LLM empty/invalid hrefs)
            let mut scope = scope.clone();
            scope.entry_url =
                self.normalize_entry_url(&scope.entry_url, scope.url_patterns.first());

            let task = WorkerTask {
                id: format!("task-{}", scope.id),
                scope: scope.clone(),
                context: format!(
                    "Explore {} section. Entry: {}. Patterns: {:?}",
                    scope.name, scope.entry_url, scope.url_patterns
                ),
                max_iterations: calculate_iterations_for_scope(&scope),
            };
            tasks.push(task);
        }

        // If no scopes found, create a fallback scope for the current area
        if tasks.is_empty() {
            let entry_url = url::Url::parse(&self.target_url)
                .map(|u| u.path().to_string())
                .unwrap_or_else(|_| "/".to_string());

            let id = sanitize_id(&entry_url);
            let id = if id.is_empty() {
                "root".to_string()
            } else {
                id
            };

            tasks.push(WorkerTask {
                id: format!("task-{}", id),
                scope: ExplorationScope {
                    id,
                    name: "Default Scope".to_string(),
                    url_patterns: vec![entry_url.clone()],
                    entry_url,
                    max_depth: 3,
                    priority: 1,
                },
                context: "Explore default scope (no navigation patterns found)".to_string(),
                max_iterations: 30, // Default iterations
            });
        }

        // If login gate detected and authenticated exploration enabled,
        // add login task with high priority
        if analysis.has_login_gate && self.config.explore_authenticated {
            if let Some(auth_entry) = analysis.auth_entries.first() {
                let login_entry_url =
                    self.normalize_entry_url(&auth_entry.href, Some(&self.target_url));
                let login_scope = ExplorationScope {
                    id: "login".to_string(),
                    name: "Login".to_string(),
                    url_patterns: vec![
                        "/login".to_string(),
                        "/signin".to_string(),
                        "/auth".to_string(),
                    ],
                    entry_url: login_entry_url,
                    max_depth: 2,
                    priority: 0, // Highest priority
                };

                tasks.insert(
                    0,
                    WorkerTask {
                        id: "task-login".to_string(),
                        scope: login_scope,
                        context: "Complete login process to access authenticated areas".to_string(),
                        max_iterations: 10,
                    },
                );
            }
        }

        Ok(tasks)
    }

    /// Normalize an entry URL to something navigable.
    /// - Empty / "#" / "javascript:" -> fallback
    /// - Relative -> join with target_url
    fn normalize_entry_url(&self, raw: &str, fallback: Option<&String>) -> String {
        let s = raw.trim();
        let is_invalid = s.is_empty() || s == "#" || s.to_lowercase().starts_with("javascript:");
        if is_invalid {
            return fallback
                .map(|v| v.clone())
                .unwrap_or_else(|| self.target_url.clone());
        }

        // Prefer absolute URLs; otherwise join with target_url.
        if s.starts_with("http://") || s.starts_with("https://") {
            return s.to_string();
        }

        self.build_full_url(s)
    }

    /// Build full URL from relative path
    fn build_full_url(&self, path: &str) -> String {
        if path.starts_with("http://") || path.starts_with("https://") {
            return path.to_string();
        }

        if let Ok(base) = url::Url::parse(&self.target_url) {
            if let Ok(full) = base.join(path) {
                return full.to_string();
            }
        }

        format!("{}{}", self.target_url.trim_end_matches('/'), path)
    }

    /// Get navigation analysis result
    pub fn get_analysis(&self) -> Option<&NavigationAnalysis> {
        self.nav_analysis.as_ref()
    }

    /// Handle cross-scope discoveries from workers
    pub async fn handle_cross_scope_discoveries(&self) -> Vec<WorkerTask> {
        let links = self.global_state.drain_cross_scope_links().await;

        if links.is_empty() {
            return Vec::new();
        }

        info!(
            "ManagerAgent: Processing {} cross-scope discoveries",
            links.len()
        );

        // Group by target scope
        let mut new_tasks = Vec::new();
        let mut seen_scopes = std::collections::HashSet::new();

        for link in links {
            if let Some(scope_id) = &link.target_scope {
                if seen_scopes.contains(scope_id) {
                    continue;
                }
                seen_scopes.insert(scope_id.clone());

                // Create new task for this scope if not already covered
                let task = WorkerTask {
                    id: format!("task-crossscope-{}", scope_id),
                    scope: ExplorationScope {
                        id: scope_id.clone(),
                        name: scope_id.clone(),
                        url_patterns: vec![link.target_url.clone()],
                        entry_url: link.target_url.clone(),
                        max_depth: self.config.default_max_depth.saturating_sub(1),
                        priority: 50, // Lower priority than initial scopes
                    },
                    context: format!("Cross-scope discovery from {}", link.source_url),
                    max_iterations: 15,
                };
                new_tasks.push(task);
            }
        }

        new_tasks
    }
}

/// Extract JSON from LLM response (handles markdown fences)
fn extract_json(response: &str) -> String {
    // Try to find JSON within markdown code blocks
    if let Some(start) = response.find("```json") {
        let json_start = start + 7;
        if let Some(end) = response[json_start..].find("```") {
            return response[json_start..json_start + end].trim().to_string();
        }
    }

    // Try plain code blocks
    if let Some(start) = response.find("```") {
        let code_start = response[start + 3..]
            .find('\n')
            .map(|i| start + 3 + i + 1)
            .unwrap_or(start + 3);
        if let Some(end) = response[code_start..].find("```") {
            return response[code_start..code_start + end].trim().to_string();
        }
    }

    // Try to find JSON object directly
    if let Some(start) = response.find('{') {
        if let Some(end) = response.rfind('}') {
            return response[start..=end].to_string();
        }
    }

    response.trim().to_string()
}

/// Truncate string with ellipsis
fn truncate(s: &str, max_len: usize) -> String {
    if s.len() <= max_len {
        s.to_string()
    } else {
        format!("{}...", &s[..max_len.saturating_sub(3)])
    }
}

/// Sanitize string to valid ID
fn sanitize_id(s: &str) -> String {
    s.to_lowercase()
        .chars()
        .filter_map(|c| {
            if c.is_alphanumeric() {
                Some(c)
            } else if c.is_whitespace() || c == '-' || c == '_' {
                Some('_')
            } else {
                None
            }
        })
        .collect::<String>()
        .trim_matches('_')
        .to_string()
}

/// Calculate iterations based on scope characteristics
fn calculate_iterations_for_scope(scope: &ExplorationScope) -> u32 {
    // Base iterations
    let base = 20u32;

    // Adjust by depth
    let depth_factor = scope.max_depth as u32 * 5;

    // Adjust by pattern count
    let pattern_factor = (scope.url_patterns.len() as u32).saturating_sub(1) * 5;

    (base + depth_factor + pattern_factor).min(50)
}

/// Spatial regions for navigation analysis
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
enum SpatialRegion {
    TopHeader,
    LeftSidebar,
    RightSidebar,
    Footer,
    MainContent,
}

/// Partition elements into spatial regions
fn partition_elements(
    elements: Vec<AnnotatedElement>,
    viewport: Option<&ViewportSize>,
) -> Vec<(SpatialRegion, Vec<AnnotatedElement>)> {
    let mut partitioned: std::collections::HashMap<SpatialRegion, Vec<AnnotatedElement>> =
        std::collections::HashMap::new();

    // Use actual viewport or fallback
    let (vw, vh) = if let Some(vp) = viewport {
        (vp.width as f64, vp.height as f64)
    } else {
        (1920.0, 910.0)
    };

    for el in elements {
        let x = el.bounding_box.x;
        let y = el.bounding_box.y;

        // Dynamic partitioning based on viewport percentage
        let region = if y < vh * 0.1 {
            // Top 10% is usually header
            SpatialRegion::TopHeader
        } else if x < vw * 0.2 {
            // Left 20% is usually sidebar
            SpatialRegion::LeftSidebar
        } else if x > vw * 0.8 {
            // Right 20%
            SpatialRegion::RightSidebar
        } else if y > vh * 0.9 {
            // Bottom 10%
            SpatialRegion::Footer
        } else {
            SpatialRegion::MainContent
        };

        partitioned.entry(region).or_default().push(el);
    }

    // Return in specific order for prompt consistency
    let order = [
        SpatialRegion::TopHeader,
        SpatialRegion::LeftSidebar,
        SpatialRegion::RightSidebar,
        SpatialRegion::MainContent,
        SpatialRegion::Footer,
    ];

    order
        .iter()
        .map(|r| (*r, partitioned.remove(r).unwrap_or_default()))
        .collect()
}
