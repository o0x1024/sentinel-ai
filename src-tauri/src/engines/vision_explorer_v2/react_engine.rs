//! ReAct Engine - Main exploration loop
//!
//! Implements the ReAct (Reasoning + Acting) pattern:
//! 1. Observe: Analyze current page state
//! 2. Think: Use LLM to decide next action
//! 3. Act: Execute the chosen action
//! 4. Update: Record results and update state

use super::action_executor::ActionExecutor;
use super::graph::ExplorationGraph;
use super::perception::PerceptionEngine;
use super::types::*;
use crate::engines::LlmClient;
use crate::services::mcp::McpService;
use anyhow::{Context, Result};
use std::sync::Arc;
use tracing::{debug, error, info};

/// ReAct exploration engine
pub struct ReActEngine {
    config: VisionExplorerV2Config,
    state: ExplorationState,
    graph: ExplorationGraph,
    perception_engine: Arc<PerceptionEngine>,
    action_executor: Arc<ActionExecutor>,
    reasoning_llm: LlmClient,
    session_id: String,
    message_callback: Option<Arc<dyn Fn(VisionMessage) + Send + Sync>>,
}

impl ReActEngine {
    /// Create a new ReAct engine
    pub fn new(
        config: VisionExplorerV2Config,
        mcp_service: Arc<McpService>,
        mcp_server_name: String,
    ) -> Self {
        let perception_engine = Arc::new(PerceptionEngine::new(
            config.ai_config.vision_llm_config(),
        ));

        let action_executor = Arc::new(ActionExecutor::new(
            mcp_service.clone(),
            perception_engine.clone(),
            mcp_server_name,
        ));

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
            perception_engine,
            action_executor,
            reasoning_llm,
            session_id,
            message_callback: None,
        }
    }

    /// Set message callback for UI updates
    pub fn with_message_callback<F>(mut self, callback: F) -> Self
    where
        F: Fn(VisionMessage) + Send + Sync + 'static,
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
        
        self.send_message(VisionMessage::Started {
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
            self.send_message(VisionMessage::Progress {
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
        };

        self.send_message(VisionMessage::Completed {
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
        self.send_message(VisionMessage::Analysis {
            step_number,
            page_type: observation.page_type.clone(),
            description: observation.description.clone(),
            elements_count: observation.elements.len(),
            forms_count: observation.forms.len(),
            links_count: observation.links.len(),
        });

        // 2. THINK: Decide next action using LLM
        let decision = self.think(&observation).await?;
        
        self.send_message(VisionMessage::Step {
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
        self.send_message(VisionMessage::ActionExecuting {
            step_number,
            action_type: self.action_type_name(&decision.action),
            action_details: self.action_to_json(&decision.action),
        });

        let action_result = self.action_executor.execute(decision.action.clone()).await?;

        // Send action result
        self.send_message(VisionMessage::ActionResult {
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
    async fn observe(&mut self) -> Result<Observation> {
        debug!("Observing current page");
        
        let step_number = self.state.steps_taken + 1;
        
        // Capture page context
        let context = self.action_executor.capture_page_context().await?;
        
        // Send screenshot to UI
        use base64::{Engine, engine::general_purpose::STANDARD as BASE64};
        let screenshot_base64 = BASE64.encode(&context.screenshot);
        self.send_message(VisionMessage::Screenshot {
            step_number,
            screenshot_base64,
            url: context.url.clone(),
            title: context.title.clone(),
        });
        
        // Analyze with perception engine
        let observation = self.perception_engine.analyze(&context).await?;
        
        Ok(observation)
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

        // Add discovered APIs
        for api in &observation.api_endpoints {
            self.state.add_api(api.clone());
            self.send_message(VisionMessage::ApiDiscovered {
                url: api.clone(),
                method: "Unknown".to_string(),
            });
        }

        // Update graph
        // TODO: Add nodes and edges based on navigation

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
1. Reason about what you observe
2. Decide on the next action
3. Explain why you chose that action

Available actions:
- navigate: Go to a new URL
- click: Click an element (by selector or coordinates)
- fill: Fill a form field
- submit: Submit a form
- scroll: Scroll the page (up/down/left/right)
- wait: Wait for content to load
- take_snapshot: Record current state
- go_back: Return to previous page
- stop: End exploration

Return your decision in JSON format:
{
  "thought": "Your reasoning about the current state and what to do next",
  "action": {
    "type": "action_type",
    "params": { /* action parameters */ }
  },
  "reason": "Why you chose this specific action"
}

Examples:
{
  "thought": "I see a login form. I should explore other pages first before dealing with authentication.",
  "action": {"type": "click", "params": {"selector": "a[href='/about']"}},
  "reason": "Clicking the About link to discover more pages without authentication"
}

{
  "thought": "I've explored all visible links on this page. Time to move on.",
  "action": {"type": "stop", "params": {"reason": "All accessible pages have been visited"}},
  "reason": "Exploration complete"
}"#.to_string()
    }

    /// Build user prompt with current observation
    fn build_thinking_user_prompt(&self, observation: &Observation) -> String {
        let recent_history = self.get_recent_history_summary(3);
        
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
- Elements: {} interactive elements found
- Forms: {} forms detected
- Links: {} links discovered
- APIs: {} endpoints found

Interactive Elements:
{}

Forms:
{}

Links:
{}

Recent History (last 3 steps):
{}

Decide what to do next."#,
            self.state.current_url,
            self.state.steps_taken,
            self.state.max_steps,
            self.state.visited_urls.len(),
            self.state.current_depth,
            self.state.max_depth,
            observation.page_type,
            observation.description,
            observation.auth_status,
            observation.elements.len(),
            observation.forms.len(),
            observation.links.len(),
            observation.api_endpoints.len(),
            self.format_elements(&observation.elements),
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

        let reason = parsed["reason"]
            .as_str()
            .unwrap_or("No reason provided")
            .to_string();

        let action = self.parse_action(&parsed["action"])?;

        Ok(ReActDecision {
            thought,
            action,
            reason,
        })
    }

    /// Parse action from JSON (supports hybrid mode with index)
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
            "click" => Action::Click {
                // Priority: index > selector > coordinates
                index: params["index"].as_u64().map(|n| n as u32),
                selector: params["selector"].as_str().map(|s| s.to_string()),
                x: params["x"].as_i64().map(|n| n as i32),
                y: params["y"].as_i64().map(|n| n as i32),
            },
            "fill" => Action::Fill {
                // Priority: index > selector
                index: params["index"].as_u64().map(|n| n as u32),
                selector: params["selector"].as_str().map(|s| s.to_string()),
                value: params["value"]
                    .as_str()
                    .context("Fill requires value")?
                    .to_string(),
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
    fn send_message(&self, message: VisionMessage) {
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
}
