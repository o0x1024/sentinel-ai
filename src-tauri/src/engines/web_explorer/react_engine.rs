//! ReAct Engine - Main exploration loop
//!
//! Implements the ReAct (Reasoning + Acting) pattern:
//! 1. Observe: Analyze current page state (using snapshot)
//! 2. Think: Use LLM to decide next action
//! 3. Act: Execute the chosen action via AgentBrowserService
//! 4. Update: Record results and update state

use super::action_executor::ActionExecutor;
use super::graph::ExplorationGraph;
use super::types::*;
use crate::engines::LlmClient;
use anyhow::{Context, Result};
use sentinel_tools::agent_browser::get_browser_service;
use std::sync::Arc;
use tracing::{debug, error, info, warn};

/// ReAct exploration engine
pub struct ReActEngine {
    config: WebExplorerConfig,
    state: ExplorationState,
    graph: ExplorationGraph,
    action_executor: Arc<ActionExecutor>,
    reasoning_llm: LlmClient,
    session_id: String,
    message_callback: Option<Arc<dyn Fn(WebExplorerMessage) + Send + Sync>>,
}

impl ReActEngine {
    /// Create a new ReAct engine (using AgentBrowserService)
    pub fn new(config: WebExplorerConfig) -> Self {
        let action_executor = Arc::new(ActionExecutor::new());
        let reasoning_llm = LlmClient::new(config.ai_config.fast_llm_config());

        let state = ExplorationState::new(
            config.target_url.clone(),
            config.max_depth,
            config.max_steps,
        );

        let graph = ExplorationGraph::new();
        let session_id = uuid::Uuid::new_v4().to_string();

        Self {
            config,
            state,
            graph,
            action_executor,
            reasoning_llm,
            session_id,
            message_callback: None,
        }
    }

    /// Set message callback for UI updates
    pub fn with_message_callback<F>(mut self, callback: F) -> Self
    where
        F: Fn(WebExplorerMessage) + Send + Sync + 'static,
    {
        self.message_callback = Some(Arc::new(callback));
        self
    }

    /// Get session ID
    pub fn session_id(&self) -> &str {
        &self.session_id
    }

    /// Start the exploration
    pub async fn run(&mut self) -> Result<ExplorationResult> {
        info!("Starting ReAct exploration: {}", self.config.target_url);

        // Ensure browser visibility matches the Vision config.
        // Vision exploration should typically be headed (headless=false), but this also
        // fixes cases where the global browser service was previously initialized headless.
        {
            let service = get_browser_service().await;
            let mut service = service.write().await;
            if let Err(e) = service.set_headless(self.config.headless).await {
                error!("Failed to set browser headless mode: {}", e);
            }
        }
        
        self.send_message(WebExplorerMessage::Started {
            session_id: self.session_id.clone(),
            target_url: self.config.target_url.clone(),
        });

        let start_time = std::time::Instant::now();

        // Initialize: Navigate to start URL
        let init_action = Action::Navigate {
            url: self.config.target_url.clone(),
        };
        
        match self.action_executor.execute(init_action).await {
            Ok(result) if result.success => {
                info!("Initial navigation successful");
                if let Err(e) = self.action_executor.enable_network_interception().await {
                    error!("Failed to enable network interception: {}", e);
                }
            }
            Ok(result) => {
                let error_msg = result.error.unwrap_or_else(|| "Unknown error".to_string());
                error!("Initial navigation failed: {}", error_msg);
                return Ok(ExplorationResult {
                    success: false,
                    pages_visited: 0,
                    apis_discovered: 0,
                    actions_performed: 0,
                    duration_seconds: start_time.elapsed().as_secs(),
                    error: Some(error_msg),
                    graph: self.graph.to_json(),
                    api_list: Vec::new(),
                    visited_urls: Vec::new(),
                });
            }
            Err(e) => {
                error!("Initial navigation error: {}", e);
                return Ok(ExplorationResult {
                    success: false,
                    pages_visited: 0,
                    apis_discovered: 0,
                    actions_performed: 0,
                    duration_seconds: start_time.elapsed().as_secs(),
                    error: Some(e.to_string()),
                    graph: self.graph.to_json(),
                    api_list: Vec::new(),
                    visited_urls: Vec::new(),
                });
            }
        }

        // Main ReAct loop
        while self.state.should_continue() {
            match self.react_step().await {
                Ok(should_continue) => {
                    if !should_continue {
                        info!("ReAct loop terminated by action");
                        break;
                    }
                }
                Err(e) => {
                    error!("Error in ReAct step: {}", e);
                    self.state.complete(format!("Error: {}", e));
                    break;
                }
            }

            // Send progress update
            self.send_message(WebExplorerMessage::Progress {
                steps_taken: self.state.steps_taken,
                max_steps: self.state.max_steps,
                pages_visited: self.state.visited_urls.len() as u32,
                apis_discovered: self.state.discovered_apis.len() as u32,
            });
        }

        // Finalize
        if !self.state.is_complete {
            self.state.complete("Max steps or depth reached".to_string());
        }

        let duration = start_time.elapsed().as_secs();
        let result = ExplorationResult {
            success: true,
            pages_visited: self.state.visited_urls.len() as u32,
            apis_discovered: self.state.discovered_apis.len() as u32,
            actions_performed: self.state.steps_taken,
            duration_seconds: duration,
            error: None,
            graph: self.graph.to_json(),
            api_list: self.state.discovered_apis.iter().cloned().collect(),
            visited_urls: self.state.visited_urls.iter().cloned().collect(),
        };

        self.send_message(WebExplorerMessage::Completed {
            success: true,
            result: result.clone(),
        });

        info!("Exploration completed: {} pages, {} APIs, {} actions",
            result.pages_visited, result.apis_discovered, result.actions_performed);

        Ok(result)
    }

    /// Execute one ReAct step
    async fn react_step(&mut self) -> Result<bool> {
        let step_number = self.state.steps_taken + 1;
        debug!("ReAct step {}", step_number);

        // 1. OBSERVE: Analyze current page
        let observation = self.observe().await?;
        
        // Send analysis result to UI
        self.send_message(WebExplorerMessage::Analysis {
            step_number,
            page_type: observation.page_type.clone(),
            description: observation.description.clone(),
            elements_count: observation.elements.len(),
            forms_count: observation.forms.len(),
            links_count: observation.links.len(),
        });

        // 2. THINK: Decide next action using LLM
        let decision = self.think(&observation).await?;
        
        self.send_message(WebExplorerMessage::Step {
            step_number,
            thought: decision.thought.clone(),
            action: format!("{:?}", decision.action),
            current_url: self.state.current_url.clone(),
        });

        // Check if should stop
        if matches!(decision.action, Action::Stop { .. }) {
            if let Action::Stop { reason } = &decision.action {
                self.state.complete(reason.clone());
            }
            return Ok(false);
        }

        // 3. ACT: Execute the action
        // Send action executing message
        self.send_message(WebExplorerMessage::ActionExecuting {
            step_number,
            action_type: self.action_type_name(&decision.action),
            action_details: self.action_to_json(&decision.action),
        });

        let action_result = self.action_executor.execute(decision.action.clone()).await?;

        // Send action result
        self.send_message(WebExplorerMessage::ActionResult {
            step_number,
            success: action_result.success,
            error: action_result.error.clone(),
            new_url: action_result.new_url.clone(),
        });

        // 4. UPDATE: Record results
        self.update_state(&observation, &decision, &action_result).await?;

        Ok(true)
    }

    /// Observe: Analyze current page state
    /// Takes a snapshot to build the refMap for element references (@e1, @e2, etc.)
    /// Snapshot provides all needed info: ARIA tree, element refs, and page structure
    async fn observe(&mut self) -> Result<Observation> {
        debug!("Observing current page");
        
        let step_number = self.state.steps_taken + 1;
        
        // Take snapshot - this is the only browser call needed
        // Snapshot provides: ARIA tree with @e1, @e2 refs for LLM to use
        let snapshot = match tokio::time::timeout(
            tokio::time::Duration::from_secs(10),
            self.action_executor.get_snapshot()
        ).await {
            Ok(Ok(s)) => {
                debug!("Snapshot taken with {} refs", s.refs.len());
                s
            }
            Ok(Err(e)) => {
                warn!("Failed to get snapshot: {}", e);
                sentinel_tools::agent_browser::Snapshot {
                    tree: String::new(),
                    refs: std::collections::HashMap::new(),
                }
            }
            Err(_) => {
                warn!("Snapshot timed out after 10s");
                sentinel_tools::agent_browser::Snapshot {
                    tree: String::new(),
                    refs: std::collections::HashMap::new(),
                }
            }
        };
        
        // Send observation message to UI (no screenshot needed)
        self.send_message(WebExplorerMessage::Screenshot {
            step_number,
            screenshot_base64: String::new(), // Empty - screenshot disabled
            url: self.state.current_url.clone(),
            title: String::new(),
        });
        
        // Build observation directly from snapshot
        // No need for separate page context capture - snapshot has everything
        let observation = self.build_observation_from_snapshot(&snapshot);
        
        // Log snapshot tree for debugging
        if !snapshot.tree.is_empty() {
            debug!("Snapshot tree (first 500 chars): {}", &snapshot.tree[..snapshot.tree.len().min(500)]);
        } else {
            warn!("Snapshot tree is empty - LLM may not have element refs");
        }
        
        Ok(observation)
    }
    
    /// Build observation directly from snapshot without additional browser calls
    fn build_observation_from_snapshot(&self, snapshot: &sentinel_tools::agent_browser::Snapshot) -> super::types::Observation {
        use super::types::*;
        
        // Analyze snapshot tree to determine page type
        let tree = &snapshot.tree;
        let page_type = if tree.contains("password") || tree.contains("login") || tree.contains("Login") {
            PageType::Login
        } else if tree.contains("error") || tree.contains("Error") {
            PageType::Error
        } else if tree.contains("form") || tree.contains("Form") {
            PageType::Form
        } else {
            PageType::Unknown
        };
        
        // Check auth status from snapshot
        let auth_status = if tree.contains("logout") || tree.contains("Logout") || tree.contains("sign out") {
            AuthStatus::Authenticated { username: None }
        } else if page_type == PageType::Login {
            AuthStatus::NotAuthenticated
        } else {
            AuthStatus::Unknown
        };
        
        // Extract links from snapshot refs
        let links: Vec<String> = snapshot.refs.iter()
            .filter(|(_, data)| data.role == "link")
            .filter_map(|(_, data)| data.name.clone())
            .collect();
        
        // Build elements from snapshot refs
        let elements: Vec<Element> = snapshot.refs.iter()
            .map(|(ref_id, ref_data)| {
                Element {
                    element_id: format!("@e{}", ref_id),
                    element_type: ref_data.role.clone(),
                    selector: format!("@e{}", ref_id),
                    text: ref_data.name.clone(),
                    href: None,
                    x: None,
                    y: None,
                    width: None,
                    height: None,
                    is_visible: true,
                }
            })
            .collect();
        
        Observation {
            page_type,
            description: format!("Page with {} interactive elements", elements.len()),
            auth_status,
            elements,
            forms: Vec::new(), // Forms are handled via snapshot refs
            links,
            api_endpoints: Vec::new(),
            confidence: 0.8,
            metadata: std::collections::HashMap::new(),
            snapshot_tree: Some(snapshot.tree.clone()),
        }
    }

    /// Think: Use LLM to decide next action
    async fn think(&self, observation: &Observation) -> Result<ReActDecision> {
        debug!("Thinking about next action");

        let system_prompt = self.build_thinking_system_prompt();
        let user_prompt = self.build_thinking_user_prompt(observation);

        let response = self
            .reasoning_llm
            .chat(Some(&system_prompt), &user_prompt, &[], None)
            .await
            .context("Failed to call LLM for reasoning")?;

        self.parse_decision(&response)
    }

    /// Update state after action
    async fn update_state(
        &mut self,
        observation: &Observation,
        decision: &ReActDecision,
        result: &ActionResult,
    ) -> Result<()> {
        let prev_url = self.state.current_url.clone();
        let prev_depth = self.state.current_depth;

        // Record step in history
        let step = Step {
            step_number: self.state.steps_taken + 1,
            observation: observation.clone(),
            thought: decision.thought.clone(),
            action: decision.action.clone(),
            result: result.clone(),
            timestamp: std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_millis() as u64,
        };

        self.state.record_step(step);

        // Update current URL if changed
        if let Some(ref new_url) = result.new_url {
            self.state.current_url = new_url.clone();
            self.state.mark_visited(new_url.clone());
        }

        // Update depth based on action and navigation
        if matches!(decision.action, Action::GoBack) {
            if self.state.current_depth > 0 {
                self.state.current_depth -= 1;
            }
        } else if let Some(ref new_url) = result.new_url {
            if new_url != &prev_url {
                self.state.current_depth = prev_depth.saturating_add(1);
            }
        }

        // Add discovered APIs from observation
        for api in &observation.api_endpoints {
            self.state.add_api(api.clone());
            // Extract method from "METHOD URL" format if possible
            let (method, url) = Self::parse_api_string(api);
            self.send_message(WebExplorerMessage::ApiDiscovered {
                url,
                method,
            });
        }

        // Merge APIs discovered from network interception
        if let Ok(apis) = self.action_executor.get_discovered_apis().await {
            for api in apis {
                // Only send message if this is a new API (not already in the list)
                let is_new = !self.state.discovered_apis.contains(&api);
                self.state.add_api(api.clone());
                
                if is_new {
                    // API format is "METHOD URL"
                    let (method, url) = Self::parse_api_string(&api);
                    debug!("Sending API discovered: method={}, url={}", method, url);
                    self.send_message(WebExplorerMessage::ApiDiscovered {
                        url,
                        method,
                    });
                }
            }
        }

        // Check if we've navigated outside target domain - return if so
        if let Some(ref new_url) = result.new_url {
            if !self.is_same_domain(new_url) {
                warn!("Navigated outside target domain: {} -> {}", self.config.target_url, new_url);
                info!("Returning to target domain...");
                let _ = self.action_executor.execute(Action::Navigate {
                    url: self.config.target_url.clone(),
                }).await;
                self.state.current_url = self.config.target_url.clone();
            }
        }

        // Update graph
        let to_url = self.state.current_url.clone();
        let from_id = format!("{}#{}", prev_url, self.state.steps_taken.saturating_sub(1));
        let to_id = format!("{}#{}", to_url, self.state.steps_taken);
        if !self.graph.has_node(&from_id) {
            self.graph.add_node(
                from_id.clone(),
                prev_url.clone(),
                observation.description.clone(),
                format!("{:?}", observation.page_type).to_lowercase(),
                prev_depth,
            );
        }
        if !self.graph.has_node(&to_id) {
            self.graph.add_node(
                to_id.clone(),
                to_url.clone(),
                observation.description.clone(),
                format!("{:?}", observation.page_type).to_lowercase(),
                self.state.current_depth,
            );
        }
        self.graph.add_edge(
            from_id,
            to_id,
            format!("{:?}", decision.action).to_lowercase(),
        );

        Ok(())
    }

    /// Build system prompt for thinking
    fn build_thinking_system_prompt(&self) -> String {
        r#"You are a web exploration agent using the ReAct (Reasoning + Acting) approach.

Your goal is to systematically explore a web application to discover:
- All accessible pages
- API endpoints
- Forms and their functionalities
- Navigation structure

For each step, you must:
1. Analyze the current page state
2. Decide on the next action

Available actions:
- navigate: Go to a new URL (params: {"url": "..."})
- click: Click an element by ref (params: {"ref": "@e5"})
- fill: Fill a form field by ref (params: {"ref": "@e3", "value": "text"})
- submit: Submit a form (params: {"selector": "form"})
- scroll: Scroll the page (params: {"direction": "up|down|left|right", "amount": 500})
- wait: Wait for content to load (params: {"duration_ms": 1000})
- take_snapshot: Record current state (params: {})
- go_back: Return to previous page (params: {})
- stop: End exploration (params: {"reason": "..."})

IMPORTANT: For click and fill actions, use the @eN refs from the snapshot tree.
Example: If you see "- @e5 link 'Products'" in the snapshot, use {"ref": "@e5"} to click it.

Return your decision in JSON format:
{
  "thought": "Your analysis of the current page and what action to take next",
  "action": {
    "type": "action_type",
    "params": { /* action parameters */ }
  }
}

Examples:
{
  "thought": "I see a Products link at @e5. Clicking it to explore and discover more pages.",
  "action": {"type": "click", "params": {"ref": "@e5"}}
}

{
  "thought": "Found a search input at @e3. Testing search functionality by entering a query.",
  "action": {"type": "fill", "params": {"ref": "@e3", "value": "test"}}
}

{
  "thought": "All visible links on this page have been explored. Exploration complete.",
  "action": {"type": "stop", "params": {"reason": "All accessible pages have been visited"}}
}"#.to_string()
    }

    /// Build user prompt with current observation
    fn build_thinking_user_prompt(&self, observation: &Observation) -> String {
        let recent_history = self.get_recent_history_summary(3);
        
        // Use snapshot tree if available, otherwise fall back to formatted elements
        let elements_section = if let Some(ref tree) = observation.snapshot_tree {
            // Truncate tree if too long
            if tree.len() > 3000 {
                format!("{}...\n(truncated)", &tree[..3000])
            } else {
                tree.clone()
            }
        } else {
            self.format_elements(&observation.elements)
        };
        
        format!(
            r#"Current State:
URL: {}
Step: {}/{}
Pages visited: {}
Depth: {}/{}

Current Observation:
- Page Type: {:?}
- Description: {}
- Auth Status: {:?}
- Forms: {} forms detected
- Links: {} links discovered
- APIs: {} endpoints found

Page Snapshot (use @eN refs for click/fill actions):
{}

Forms:
{}

Links:
{}

Recent History (last 3 steps):
{}

Decide what to do next. Use @eN refs from the snapshot for click/fill actions.
**IMPORTANT**: You must answer in Chinese (Simplified Chinese)"#,
            self.state.current_url,
            self.state.steps_taken,
            self.state.max_steps,
            self.state.visited_urls.len(),
            self.state.current_depth,
            self.state.max_depth,
            observation.page_type,
            observation.description,
            observation.auth_status,
            observation.forms.len(),
            observation.links.len(),
            self.state.discovered_apis.len(),  // Use state's discovered_apis instead of observation
            elements_section,
            self.format_forms(&observation.forms),
            self.format_links(&observation.links),
            recent_history
        )
    }

    /// Format elements for prompt
    fn format_elements(&self, elements: &[Element]) -> String {
        elements
            .iter()
            .take(10)
            .map(|e| {
                format!(
                    "  - {} [{}]: {} (selector: {})",
                    e.element_type,
                    e.element_id,
                    e.text.as_deref().unwrap_or(""),
                    e.selector
                )
            })
            .collect::<Vec<_>>()
            .join("\n")
    }

    /// Format forms for prompt
    fn format_forms(&self, forms: &[FormInfo]) -> String {
        forms
            .iter()
            .map(|f| {
                let fields = f
                    .fields
                    .iter()
                    .map(|field| format!("{}:{}", field.name, field.field_type))
                    .collect::<Vec<_>>()
                    .join(", ");
                format!("  - Form [{}]: {}", f.selector, fields)
            })
            .collect::<Vec<_>>()
            .join("\n")
    }

    /// Format links for prompt
    fn format_links(&self, links: &[String]) -> String {
        links
            .iter()
            .take(10)
            .map(|l| format!("  - {}", l))
            .collect::<Vec<_>>()
            .join("\n")
    }

    /// Get recent history summary
    fn get_recent_history_summary(&self, count: usize) -> String {
        let start = if self.state.history.len() > count {
            self.state.history.len() - count
        } else {
            0
        };

        self.state.history[start..]
            .iter()
            .map(|step| {
                format!(
                    "Step {}: {} -> {:?}",
                    step.step_number,
                    step.thought.chars().take(50).collect::<String>(),
                    step.action
                )
            })
            .collect::<Vec<_>>()
            .join("\n")
    }

    /// Parse LLM decision response
    fn parse_decision(&self, response: &str) -> Result<ReActDecision> {
        // Extract JSON from response
        let json_str = self.extract_json(response)?;
        let parsed: serde_json::Value = serde_json::from_str(&json_str)
            .context("Failed to parse decision JSON")?;

        let thought = parsed["thought"]
            .as_str()
            .unwrap_or("No thought provided")
            .to_string();

        let action = self.parse_action(&parsed["action"])?;

        Ok(ReActDecision {
            thought,
            action,
        })
    }

    /// Parse action from JSON (supports ref like @e5)
    fn parse_action(&self, json: &serde_json::Value) -> Result<Action> {
        let action_type = json["type"]
            .as_str()
            .context("Action type not specified")?;

        let params = &json["params"];

        let action = match action_type {
            "navigate" => Action::Navigate {
                url: params["url"]
                    .as_str()
                    .context("Navigate requires url")?
                    .to_string(),
            },
            "click" => {
                // Support ref (@e5), index, selector, or coordinates
                // Priority: ref > index > selector > coordinates
                let (index, selector) = Self::parse_ref_or_index(params);
                Action::Click {
                    index,
                    selector,
                    x: params["x"].as_i64().map(|n| n as i32),
                    y: params["y"].as_i64().map(|n| n as i32),
                }
            },
            "fill" => {
                // Support ref (@e3) or index or selector
                let (index, selector) = Self::parse_ref_or_index(params);
                Action::Fill {
                    index,
                    selector,
                    value: params["value"]
                        .as_str()
                        .context("Fill requires value")?
                        .to_string(),
                }
            },
            "submit" => Action::Submit {
                selector: params["selector"]
                    .as_str()
                    .context("Submit requires selector")?
                    .to_string(),
            },
            "scroll" => {
                let direction = match params["direction"].as_str() {
                    Some("up") => ScrollDirection::Up,
                    Some("down") => ScrollDirection::Down,
                    Some("left") => ScrollDirection::Left,
                    Some("right") => ScrollDirection::Right,
                    _ => ScrollDirection::Down,
                };
                Action::Scroll {
                    direction,
                    amount: params["amount"].as_u64().unwrap_or(300) as u32,
                }
            }
            "wait" => Action::Wait {
                duration_ms: params["duration_ms"].as_u64().unwrap_or(1000),
            },
            "take_snapshot" => Action::TakeSnapshot,
            "go_back" => Action::GoBack,
            "stop" => Action::Stop {
                reason: params["reason"]
                    .as_str()
                    .unwrap_or("Exploration complete")
                    .to_string(),
            },
            _ => anyhow::bail!("Unknown action type: {}", action_type),
        };

        Ok(action)
    }

    /// Extract JSON from LLM response
    fn extract_json(&self, response: &str) -> Result<String> {
        let trimmed = response.trim();
        
        if trimmed.starts_with("```") {
            let lines: Vec<&str> = trimmed.lines().collect();
            if lines.len() > 2 {
                let json_lines = &lines[1..lines.len() - 1];
                return Ok(json_lines.join("\n"));
            }
        }
        
        if let Some(start) = trimmed.find('{') {
            if let Some(end) = trimmed.rfind('}') {
                return Ok(trimmed[start..=end].to_string());
            }
        }
        
        Ok(trimmed.to_string())
    }

    /// Send message to UI
    fn send_message(&self, message: WebExplorerMessage) {
        if let Some(ref callback) = self.message_callback {
            callback(message);
        }
    }

    /// Get action type name
    fn action_type_name(&self, action: &Action) -> String {
        match action {
            Action::Navigate { .. } => "navigate".to_string(),
            Action::Click { .. } => "click".to_string(),
            Action::Fill { .. } => "fill".to_string(),
            Action::Submit { .. } => "submit".to_string(),
            Action::Scroll { .. } => "scroll".to_string(),
            Action::Wait { .. } => "wait".to_string(),
            Action::TakeSnapshot => "snapshot".to_string(),
            Action::GoBack => "go_back".to_string(),
            Action::Stop { .. } => "stop".to_string(),
        }
    }

    /// Convert action to JSON for UI
    fn action_to_json(&self, action: &Action) -> serde_json::Value {
        match action {
            Action::Navigate { url } => serde_json::json!({
                "url": url
            }),
            Action::Click { index, selector, x, y } => serde_json::json!({
                "index": index,
                "selector": selector,
                "x": x,
                "y": y
            }),
            Action::Fill { index, selector, value } => serde_json::json!({
                "index": index,
                "selector": selector,
                "value": value
            }),
            Action::Submit { selector } => serde_json::json!({
                "selector": selector
            }),
            Action::Scroll { direction, amount } => serde_json::json!({
                "direction": format!("{:?}", direction).to_lowercase(),
                "amount": amount
            }),
            Action::Wait { duration_ms } => serde_json::json!({
                "duration_ms": duration_ms
            }),
            Action::TakeSnapshot => serde_json::json!({}),
            Action::GoBack => serde_json::json!({}),
            Action::Stop { reason } => serde_json::json!({
                "reason": reason
            }),
        }
    }

    /// Parse API string in "METHOD URL" format
    fn parse_api_string(api: &str) -> (String, String) {
        let parts: Vec<&str> = api.splitn(2, ' ').collect();
        if parts.len() == 2 {
            (parts[0].to_string(), parts[1].to_string())
        } else {
            // No method specified, assume GET
            ("GET".to_string(), api.to_string())
        }
    }

    /// Parse ref (@e5) or index from params
    /// Returns (index, selector) - one will be Some, other None
    fn parse_ref_or_index(params: &serde_json::Value) -> (Option<u32>, Option<String>) {
        // Check for ref first (e.g., "@e5")
        if let Some(ref_str) = params["ref"].as_str() {
            // Parse @eN format to extract index
            if let Some(num_str) = ref_str.strip_prefix("@e") {
                if let Ok(idx) = num_str.parse::<u32>() {
                    return (Some(idx), None);
                }
            }
            // If not @eN format, treat as selector
            return (None, Some(ref_str.to_string()));
        }
        
        // Check for index
        if let Some(idx) = params["index"].as_u64() {
            return (Some(idx as u32), None);
        }
        
        // Check for selector
        if let Some(sel) = params["selector"].as_str() {
            return (None, Some(sel.to_string()));
        }
        
        (None, None)
    }

    /// Check if a URL is on the same domain as the target
    fn is_same_domain(&self, url: &str) -> bool {
        // Extract domain from target URL
        let target_domain = Self::extract_domain(&self.config.target_url);
        let url_domain = Self::extract_domain(url);
        
        // Allow same domain or subdomains
        if let (Some(target), Some(current)) = (target_domain, url_domain) {
            current == target || current.ends_with(&format!(".{}", target))
        } else {
            // If we can't parse domains, allow navigation
            true
        }
    }

    /// Extract domain from URL
    fn extract_domain(url: &str) -> Option<String> {
        // Simple domain extraction
        let url = url.trim();
        
        // Remove protocol
        let without_protocol = url
            .strip_prefix("https://")
            .or_else(|| url.strip_prefix("http://"))
            .unwrap_or(url);
        
        // Get domain part (before first / or ?)
        let domain = without_protocol
            .split('/')
            .next()
            .and_then(|s| s.split('?').next())
            .and_then(|s| s.split(':').next()) // Remove port
            .map(|s| s.to_lowercase());
        
        domain
    }
}
