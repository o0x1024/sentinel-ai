//! V2Engine - The main orchestrator for Vision Explorer V2
//!
//! This engine wires together all components:
//! - Event Bus for agent communication
//! - Blackboard for shared state
//! - Safety Layer for action filtering
//! - Persistence for crash recovery
//! - All specialized Agents

use crate::engines::vision_explorer_v2::blackboard::{Blackboard, ExplorationConfig};
use crate::engines::vision_explorer_v2::brain::{AuthAgent, PlannerAgent};
use crate::engines::vision_explorer_v2::core::{Agent, Event, PageContext};
use crate::engines::vision_explorer_v2::driver::{BrowserDriver, NavigatorAgent, OperatorAgent};
use crate::engines::vision_explorer_v2::emitter::V2MessageEmitter;
use crate::engines::vision_explorer_v2::graph::ExplorationGraph;
use crate::engines::vision_explorer_v2::perception::{
    PerceptionAgent, StructuralAnalyst, VisualAnalyst,
};
use crate::engines::vision_explorer_v2::persistence::{
    ExplorationSnapshot, ExplorationStats, PersistenceManager,
};
use crate::engines::vision_explorer_v2::safety::{SafetyLayer, SafetyPolicy};
use crate::engines::vision_explorer_v2::types::VisionExplorerV2Config;
use anyhow::Result;
use std::path::PathBuf;
use std::sync::Arc;
use tokio::sync::{mpsc, RwLock};

/// The main entry point for the Vision Explorer V2.
/// Wires together the Event Bus, Agents, and World Model.
pub struct V2Engine {
    /// Configuration
    config: VisionExplorerV2Config,

    /// The exploration graph (world model)
    graph: Arc<RwLock<ExplorationGraph>>,

    /// Shared blackboard for agent communication
    blackboard: Arc<Blackboard>,

    /// Safety layer for action filtering
    safety: Arc<RwLock<SafetyLayer>>,

    /// Persistence manager for crash recovery
    persistence: Option<PersistenceManager>,

    /// Event bus sender
    event_tx: mpsc::Sender<Event>,

    /// Event bus receiver (consumed on start)
    event_rx: Option<mpsc::Receiver<Event>>,

    /// Session ID for this exploration
    session_id: String,

    /// Statistics
    stats: Arc<RwLock<ExplorationStats>>,

    /// Message emitter for frontend communication
    emitter: Option<V2MessageEmitter>,
}

impl V2Engine {
    /// Create a new V2Engine with default settings
    pub fn new(config: VisionExplorerV2Config) -> Self {
        let (tx, rx) = mpsc::channel(100);
        let graph = Arc::new(RwLock::new(ExplorationGraph::new()));
        let session_id = uuid::Uuid::new_v4().to_string();

        // Create blackboard with exploration config
        let exploration_config = ExplorationConfig {
            scope_base_url: Some(config.target_url.clone()),
            max_depth: config.max_depth,
            max_steps: config.max_steps,
            exclude_patterns: vec![r"logout".to_string(), r"signout".to_string()],
            allow_destructive: false,
            auto_fill_forms: true,
        };
        let blackboard = Arc::new(Blackboard::with_config(exploration_config));

        // Create safety layer
        let safety = Arc::new(RwLock::new(SafetyLayer::new(SafetyPolicy::default())));

        Self {
            config,
            graph,
            blackboard,
            safety,
            persistence: None,
            event_tx: tx,
            event_rx: Some(rx),
            session_id,
            stats: Arc::new(RwLock::new(ExplorationStats::default())),
            emitter: None,
        }
    }

    /// Set message emitter for frontend communication
    pub fn with_emitter(mut self, emitter: V2MessageEmitter) -> Self {
        self.emitter = Some(emitter);
        self
    }

    /// Enable persistence with the given storage directory
    pub fn with_persistence(mut self, storage_dir: impl Into<PathBuf>) -> Self {
        self.persistence = Some(PersistenceManager::new(
            storage_dir,
            self.session_id.clone(),
        ));
        self
    }

    /// Set custom safety policy
    pub fn with_safety_policy(self, policy: SafetyPolicy) -> Self {
        // We need to replace the safety layer
        let safety = Arc::new(RwLock::new(SafetyLayer::new(policy)));
        Self { safety, ..self }
    }

    /// Set credentials for auto-login
    pub async fn set_credentials(&self, username: String, password: String) {
        self.blackboard.set_credentials(username, password).await;
    }

    /// Get the session ID
    pub fn session_id(&self) -> &str {
        &self.session_id
    }

    /// Get the event sender for external control
    pub fn event_sender(&self) -> mpsc::Sender<Event> {
        self.event_tx.clone()
    }

    /// Try to resume from a previous session
    pub async fn try_resume(&mut self) -> Result<bool> {
        if let Some(ref persistence) = self.persistence {
            if let Some(snapshot) = persistence.load()? {
                log::info!(
                    "Resuming from snapshot: {} steps taken, {} nodes",
                    snapshot.steps_taken,
                    snapshot.nodes.len()
                );

                // Restore graph
                let mut graph = self.graph.write().await;
                for node in snapshot.nodes {
                    graph.add_node(node);
                }

                // Restore blackboard
                // Note: We'd need a from_data method on Blackboard
                // For now, just restore key states

                // Restore stats
                let mut stats = self.stats.write().await;
                *stats = snapshot.stats;

                return Ok(true);
            }
        }
        Ok(false)
    }

    /// Start the exploration loop
    pub async fn start(&mut self) -> Result<()> {
        let mut event_rx = self.event_rx.take().expect("Engine already started");

        // 1. Initialize LLM Configs from AIConfig
        let fast_llm_config = self.config.ai_config.fast_llm_config();
        let vision_llm_config = self.config.ai_config.vision_llm_config();

        log::info!(
            "V2Engine using Fast LLM: {} ({}), Vision LLM: {} ({})",
            self.config.ai_config.fast_model_id,
            self.config.ai_config.fast_provider,
            self.config.ai_config.vision_model_id,
            self.config.ai_config.vision_provider
        );

        // 2. Initialize Agents
        let planner = PlannerAgent::new(
            "global_planner".to_string(),
            self.graph.clone(),
            self.blackboard.clone(),
            self.config.clone(),
            self.event_tx.clone(),
        );

        // Auth Agent
        let auth_agent = AuthAgent::new(
            "auth_agent".to_string(),
            self.blackboard.clone(),
            self.event_tx.clone(),
        );

        // Browser Driver (via MCP Playwright)
        let driver_impl = BrowserDriver::new();
        let driver: Arc<
            tokio::sync::Mutex<dyn crate::engines::vision_explorer_v2::driver::BrowserActions>,
        > = Arc::new(tokio::sync::Mutex::new(driver_impl));

        let navigator = NavigatorAgent::new(
            "navigator_1".to_string(),
            driver.clone(),
            self.event_tx.clone(),
            self.blackboard.clone(),
        );

        let operator = OperatorAgent::new(
            "operator_1".to_string(),
            driver.clone(),
            self.event_tx.clone(),
        );

        // Perception Agents - use fast_llm_config for structural analysis
        let structural_analyst = Box::new(StructuralAnalyst::new(
            fast_llm_config.clone(),
            self.config.ai_config.fast_model_id.clone(),
        ));
        let analyst_agent = PerceptionAgent::new(
            "structural_analyst".to_string(),
            structural_analyst,
            self.event_tx.clone(),
        );

        // Vision Agent - use vision_llm_config for visual analysis
        let visual_analyst = Box::new(VisualAnalyst::new(
            vision_llm_config.clone(),
            self.config.ai_config.vision_model_id.clone(),
        ));
        let visual_agent = PerceptionAgent::new(
            "visual_analyst".to_string(),
            visual_analyst,
            self.event_tx.clone(),
        );

        // 3. Dispatch initial navigation
        log::info!(
            "V2Engine starting exploration of: {}",
            self.config.target_url
        );

        if let Some(ref emitter) = self.emitter {
            emitter.emit_start(&self.config.target_url);
        }

        self.event_tx
            .send(Event::TaskAssigned {
                agent_id: "navigator_1".to_string(),
                task_id: "init".to_string(),
                target_node_id: self.config.target_url.clone(),
                payload: None,
            })
            .await?;

        // 4. Main Event Loop
        log::info!("Starting V2 Engine Loop");
        let mut step_count = 0u32;

        while let Some(event) = event_rx.recv().await {
            log::debug!("V2 Event: {:?}", event);

            match &event {
                Event::Stop => {
                    log::info!("Stopping V2 Engine");
                    if let Some(ref emitter) = self.emitter {
                        let stats = self.stats.read().await;
                        emitter.emit_complete(&stats, "stopped", 0);
                    }
                    // Save final snapshot
                    self.save_snapshot().await?;
                    break;
                }

                Event::CredentialsReceived {
                    username,
                    password,
                    verification_code,
                } => {
                    log::info!("V2Engine: Credentials received for user: {}", username);
                    // Store credentials in blackboard
                    self.blackboard
                        .set_credentials(username.clone(), password.clone())
                        .await;
                    self.blackboard
                        .set_kv(
                            "verification_code".to_string(),
                            serde_json::json!(verification_code),
                        )
                        .await;

                    // Notify auth agent to attempt login
                    self.event_tx
                        .send(Event::Log {
                            level: "info".to_string(),
                            message: "Credentials stored, attempting login...".to_string(),
                        })
                        .await?;
                }

                Event::LoginTakeoverRequest { url, .. } => {
                    log::info!("V2Engine: Login takeover requested for {}", url);
                    // Start login wait
                    self.blackboard.set_login_url(url.clone()).await;
                    self.blackboard.start_login_wait().await;

                    // Notify frontend with timeout info
                    if let Some(ref emitter) = self.emitter {
                        emitter.emit_takeover_request_with_timeout(
                            &format!("Login required at {}", url),
                            Some(self.config.login_timeout_seconds),
                        );
                        emitter.emit_login_wait_status(
                            true,
                            Some(self.config.login_timeout_seconds),
                            "Waiting for user login...",
                        );
                    }
                }

                Event::SkipLogin => {
                    log::info!("V2Engine: User chose to skip login");
                    self.blackboard
                        .set_kv("skip_login".to_string(), serde_json::json!(true))
                        .await;
                    // Clear login wait state
                    self.blackboard.clear_login_wait().await;
                    if let Some(ref emitter) = self.emitter {
                        emitter.emit_login_wait_status(false, None, "Login skipped by user");
                    }
                }

                Event::ManualLoginComplete => {
                    log::info!("V2Engine: User signaled manual login complete");
                    // Mark as authenticated
                    self.blackboard.set_authenticated(true).await;
                    // Clear login wait state
                    self.blackboard.clear_login_wait().await;
                    if let Some(ref emitter) = self.emitter {
                        emitter.emit_login_wait_status(false, None, "Manual login completed");
                        emitter.emit_credentials_received("manual_login_user");
                    }
                    // The next iteration will re-capture context and continue exploration
                }

                Event::LoginTimeout { url } => {
                    log::info!("V2Engine: Login timeout expired for {}", url);
                    // Clear login wait state
                    self.blackboard.clear_login_wait().await;
                    // Set a flag to indicate LLM should attempt auto-login
                    self.blackboard
                        .set_kv(
                            "login_timeout_triggered".to_string(),
                            serde_json::json!(true),
                        )
                        .await;
                    if let Some(ref emitter) = self.emitter {
                        emitter.emit_login_wait_status(
                            false,
                            None,
                            "Login timeout - attempting auto-login",
                        );
                    }
                    // The AuthAgent will detect this flag and attempt auto-login using LLM
                }

                Event::TaskAssigned { agent_id, .. } => {
                    // Route to appropriate agent
                    if agent_id == &navigator.id() {
                        if let Err(e) = navigator.handle_event(&event).await {
                            log::error!("NavigatorAgent error: {}", e);
                            if let Some(ref emitter) = self.emitter {
                                emitter.emit_error(step_count, &format!("Navigator error: {}", e));
                            }
                        }
                    } else if agent_id == &analyst_agent.id() {
                        if let Err(e) = analyst_agent.handle_event(&event).await {
                            log::error!("StructuralAnalyst error: {}", e);
                            if let Some(ref emitter) = self.emitter {
                                emitter.emit_error(step_count, &format!("Analyst error: {}", e));
                            }
                        }
                    } else if agent_id == &visual_agent.id() {
                        if let Err(e) = visual_agent.handle_event(&event).await {
                            log::error!("VisualAnalyst error: {}", e);
                            if let Some(ref emitter) = self.emitter {
                                emitter.emit_error(
                                    step_count,
                                    &format!("Visual Analyst error: {}", e),
                                );
                            }
                        }
                    } else if agent_id == &operator.id() {
                        if let Err(e) = operator.handle_event(&event).await {
                            log::error!("OperatorAgent error: {}", e);
                            if let Some(ref emitter) = self.emitter {
                                emitter.emit_error(step_count, &format!("Operator error: {}", e));
                            }
                        }
                    } else if agent_id == &auth_agent.id() {
                        if let Err(e) = auth_agent.handle_event(&event).await {
                            log::error!("AuthAgent error: {}", e);
                            if let Some(ref emitter) = self.emitter {
                                emitter.emit_error(step_count, &format!("Auth error: {}", e));
                            }
                        }
                    }
                }

                Event::TaskCompleted {
                    agent_id, result, ..
                } => {
                    // Update statistics
                    {
                        let mut stats = self.stats.write().await;
                        if result.success {
                            stats.actions_performed += 1;
                            if agent_id.contains("navigator") {
                                stats.pages_visited += 1;
                            }
                        } else {
                            // Track failures
                            log::warn!("Task failed for agent {}: {}", agent_id, result.message);
                        }
                    }

                    // Emit to frontend
                    if let Some(ref emitter) = self.emitter {
                        if agent_id.contains("navigator") {
                            if let Some(data) = &result.data {
                                if let Ok(ctx) = serde_json::from_value::<PageContext>(data.clone())
                                {
                                    emitter.emit_screenshot(
                                        step_count,
                                        &ctx.url,
                                        &ctx.title,
                                        ctx.screenshot
                                            .as_ref()
                                            .map(|b| {
                                                base64::Engine::encode(
                                                    &base64::engine::general_purpose::STANDARD,
                                                    b,
                                                )
                                            })
                                            .as_deref(),
                                    );
                                }
                            }
                        } else if agent_id.contains("analyst") {
                            if let Some(data) = &result.data {
                                if let Ok(perception) =
                                    serde_json::from_value::<
                                        crate::engines::vision_explorer_v2::core::PerceptionResult,
                                    >(data.clone())
                                {
                                    emitter.emit_analysis(
                                        step_count,
                                        &perception.summary,
                                        &perception.suggested_actions,
                                        0.0,
                                    );
                                }
                            }
                        } else if agent_id.contains("operator") || agent_id.contains("navigator") {
                            // Actions are usually handled by Navigator/Operator
                            // In V2, we might want to emit action when it's assigned or completed
                        }

                        if !result.success {
                            emitter.emit_error(step_count, &result.message);

                            // Check for fatal errors that should stop the engine
                            if result.message.contains("Playwright MCP server not connected") {
                                log::error!("Fatal error: MCP server not connected. Stopping engine.");
                                if let Err(e) = self.event_tx.send(Event::Stop).await {
                                    log::error!("Failed to send Stop event: {}", e);
                                }
                            }
                        }
                    }

                    // Let planner process the completion (even if failed)
                    // Planner should decide whether to retry or continue with other tasks
                    if let Err(e) = planner.handle_event(&event).await {
                        log::error!("Planner error while processing TaskCompleted: {}", e);
                        // Continue anyway - don't let planner errors stop the engine
                    }

                    step_count += 1;

                    // Auto-save periodically
                    if step_count.is_multiple_of(10) {
                        let _ = self.save_snapshot().await;
                    }

                    // Check max steps
                    let config = self.blackboard.get_config().await;
                    if step_count >= config.max_steps {
                        log::info!("Reached max steps ({}), stopping", config.max_steps);
                        self.event_tx.send(Event::Stop).await?;
                    }
                }

                Event::Log { level, message } => {
                    match level.as_str() {
                        "error" => log::error!("[V2] {}", message),
                        "warn" => log::warn!("[V2] {}", message),
                        "info" => log::info!("[V2] {}", message),
                        _ => log::debug!("[V2] {}", message),
                    }
                    if let Some(ref emitter) = self.emitter {
                        emitter.emit_log(level, message);
                    }
                }

                Event::NodeDiscovered { .. } => {
                    // Could log or track discovery
                }
            }

            // Check for login timeout
            if self.blackboard.is_waiting_for_login().await {
                if let Some(started) = self.blackboard.get_login_wait_started().await {
                    let now = std::time::SystemTime::now()
                        .duration_since(std::time::UNIX_EPOCH)
                        .unwrap_or_default()
                        .as_millis() as u64;
                    let elapsed_seconds = (now - started) / 1000;

                    if elapsed_seconds >= self.config.login_timeout_seconds {
                        log::info!("Login wait timeout reached ({} seconds)", elapsed_seconds);
                        // Get login URL for the timeout event
                        let login_url = if let Some(url) = self.blackboard.get_kv("login_url").await
                        {
                            url.as_str().unwrap_or("unknown").to_string()
                        } else {
                            "unknown".to_string()
                        };

                        // Trigger timeout event
                        self.event_tx
                            .send(Event::LoginTimeout { url: login_url })
                            .await?;
                        continue; // Process the timeout event in next iteration
                    } else {
                        // Still waiting, emit remaining time update
                        let remaining = self.config.login_timeout_seconds - elapsed_seconds;
                        if elapsed_seconds.is_multiple_of(10) {
                            // Update every 10 seconds
                            if let Some(ref emitter) = self.emitter {
                                emitter.emit_login_wait_status(
                                    true,
                                    Some(remaining),
                                    &format!(
                                        "Waiting for login... {} seconds remaining",
                                        remaining
                                    ),
                                );
                            }
                        }
                    }
                }
                // While waiting for login, don't ask planner for next step
                continue;
            }

            // Ask planner for next task
            match planner.decide_next_step().await {
                Ok(Some(next_task)) => {
                    // Apply safety filter before dispatching
                    if self.is_task_safe(&next_task).await {
                        // If it's an action (TaskAssigned to navigator/operator with payload), emit it
                        if let Some(ref emitter) = self.emitter {
                            if let Event::TaskAssigned {
                                agent_id, payload, ..
                            } = &next_task
                            {
                                if (agent_id.contains("navigator") || agent_id.contains("operator"))
                                    && payload.is_some()
                                {
                                    if let Ok(action) = serde_json::from_value::<
                                        crate::engines::vision_explorer_v2::core::SuggestedAction,
                                    >(
                                        payload.as_ref().unwrap().clone()
                                    ) {
                                        emitter.emit_action(step_count, &action, true, None);
                                    }
                                }
                            }
                        }

                        if let Err(e) = self.event_tx.send(next_task).await {
                            log::error!("Failed to send next task to event bus: {}", e);
                            // Don't break the loop, just log and continue
                        }
                    } else {
                        log::warn!("Safety layer blocked a task");
                        if let Some(ref emitter) = self.emitter {
                            emitter.emit_log("warn", "Safety layer blocked a restricted action");
                        }
                    }
                }
                Ok(None) => {
                    // Planner has no next task - this is normal, might be waiting for more events
                    log::debug!("Planner has no next task at this time");
                }
                Err(e) => {
                    log::error!("Planner error while deciding next step: {}", e);
                    if let Some(ref emitter) = self.emitter {
                        emitter.emit_error(step_count, &format!("Planner error: {}", e));
                    }
                    // Continue the loop - don't let planner errors stop exploration
                }
            }
        }

        log::info!("V2 Engine Loop ended");
        Ok(())
    }

    /// Check if a task is safe to execute
    async fn is_task_safe(&self, event: &Event) -> bool {
        if let Event::TaskAssigned {
            payload,
            target_node_id,
            ..
        } = event
        {
            let safety = self.safety.read().await;

            // Check URL safety
            let url_check = safety.check_url(target_node_id);
            if !url_check.allowed {
                log::info!(
                    "Safety: Blocked URL {}: {:?}",
                    target_node_id,
                    url_check.reason
                );
                return false;
            }

            // Check action safety if there's a payload with action
            if let Some(val) = payload {
                if let Ok(action) = serde_json::from_value::<
                    crate::engines::vision_explorer_v2::core::SuggestedAction,
                >(val.clone())
                {
                    let action_check = safety.check_action(&action);
                    if !action_check.allowed {
                        log::info!(
                            "Safety: Blocked action '{}': {:?}",
                            action.description,
                            action_check.reason
                        );
                        return false;
                    }
                }
            }
        }
        true
    }

    /// Save current state to snapshot
    async fn save_snapshot(&self) -> Result<()> {
        if let Some(ref persistence) = self.persistence {
            let _graph = self.graph.read().await;
            let stats = self.stats.read().await;

            let mut snapshot =
                ExplorationSnapshot::new(self.session_id.clone(), self.config.target_url.clone());

            // Note: We'd need to extract nodes from the graph
            // For now, save stats
            snapshot.stats = stats.clone();
            snapshot.blackboard =
                serde_json::from_value(self.blackboard.to_json().await).unwrap_or_default();

            persistence.save(&snapshot)?;
            log::debug!("Saved exploration snapshot");
        }
        Ok(())
    }

    /// Get current exploration statistics
    pub async fn get_stats(&self) -> ExplorationStats {
        self.stats.read().await.clone()
    }

    /// Get the blackboard for external access
    pub fn blackboard(&self) -> Arc<Blackboard> {
        self.blackboard.clone()
    }
}
