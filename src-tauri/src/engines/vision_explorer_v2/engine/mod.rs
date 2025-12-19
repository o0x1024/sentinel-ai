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
use crate::engines::vision_explorer_v2::core::{Agent, Event};
use crate::engines::vision_explorer_v2::driver::{BrowserDriver, NavigatorAgent, OperatorAgent};
use crate::engines::vision_explorer_v2::graph::ExplorationGraph;
use crate::engines::vision_explorer_v2::perception::{
    PerceptionAgent, StructuralAnalyst, VisualAnalyst,
};
use crate::engines::vision_explorer_v2::persistence::{
    ExplorationSnapshot, ExplorationStats, PersistenceManager,
};
use crate::engines::vision_explorer_v2::safety::{SafetyLayer, SafetyPolicy};
use crate::engines::vision_explorer_v2::types::VisionExplorerV2Config;
use crate::engines::LlmConfig;
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
            max_steps: 500,
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
        }
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

                Event::LoginTakeoverRequest { url, fields } => {
                    log::info!("V2Engine: Login takeover requested for {}", url);
                    // This is typically emitted by AuthAgent when login is needed
                    // The emitter/frontend handler should pick this up
                    self.blackboard.set_login_url(url.clone()).await;
                }

                Event::SkipLogin => {
                    log::info!("V2Engine: User chose to skip login");
                    self.blackboard
                        .set_kv("skip_login".to_string(), serde_json::json!(true))
                        .await;
                }

                Event::TaskAssigned { agent_id, .. } => {
                    // Route to appropriate agent
                    if agent_id == &navigator.id() {
                        let _ = navigator.handle_event(&event).await;
                    } else if agent_id == &analyst_agent.id() {
                        let _ = analyst_agent.handle_event(&event).await;
                    } else if agent_id == &visual_agent.id() {
                        let _ = visual_agent.handle_event(&event).await;
                    } else if agent_id == &operator.id() {
                        let _ = operator.handle_event(&event).await;
                    } else if agent_id == &auth_agent.id() {
                        let _ = auth_agent.handle_event(&event).await;
                    }
                }

                Event::TaskCompleted {
                    agent_id, result, ..
                } => {
                    // Update statistics
                    {
                        let mut stats = self.stats.write().await;
                        stats.actions_performed += 1;
                        if agent_id.contains("navigator") {
                            stats.pages_visited += 1;
                        }
                    }

                    // Let planner process the completion
                    let _ = planner.handle_event(&event).await;

                    step_count += 1;

                    // Auto-save periodically
                    if step_count % 10 == 0 {
                        let _ = self.save_snapshot().await;
                    }

                    // Check max steps
                    let config = self.blackboard.get_config().await;
                    if step_count >= config.max_steps {
                        log::info!("Reached max steps ({}), stopping", config.max_steps);
                        self.event_tx.send(Event::Stop).await?;
                    }
                }

                Event::Log { level, message } => match level.as_str() {
                    "error" => log::error!("[V2] {}", message),
                    "warn" => log::warn!("[V2] {}", message),
                    "info" => log::info!("[V2] {}", message),
                    _ => log::debug!("[V2] {}", message),
                },

                Event::NodeDiscovered { .. } => {
                    // Could log or track discovery
                }
            }

            // Ask planner for next task
            if let Ok(Some(next_task)) = planner.decide_next_step().await {
                // Apply safety filter before dispatching
                if self.is_task_safe(&next_task).await {
                    self.event_tx.send(next_task).await?;
                } else {
                    log::warn!("Safety layer blocked a task");
                }
            }
        }

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
            let graph = self.graph.read().await;
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
