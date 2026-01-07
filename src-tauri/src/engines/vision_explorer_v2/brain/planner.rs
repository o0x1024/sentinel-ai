use crate::engines::vision_explorer_v2::blackboard::Blackboard;
use crate::engines::vision_explorer_v2::brain::auth_agent::AuthAgent;
use crate::engines::vision_explorer_v2::brain::pattern_solver::NavigationPatternSolver;
use crate::engines::vision_explorer_v2::core::{
    Agent, Event, PageContext, PerceptionResult,
};
use crate::engines::vision_explorer_v2::graph::{
    ExplorationGraph, ExplorationStatus, PageStateNode,
};
use crate::engines::vision_explorer_v2::VisionExplorerV2Config;
use anyhow::Result;
use async_trait::async_trait;
use std::collections::VecDeque;
use std::sync::Arc;
use tokio::sync::{mpsc, RwLock};

/// The Strategist. Manages the exploration frontier and assigns tasks.
pub struct PlannerAgent {
    id: String,
    graph: Arc<RwLock<ExplorationGraph>>,
    blackboard: Arc<Blackboard>,
    #[allow(dead_code)]
    config: VisionExplorerV2Config,
    frontier: Arc<RwLock<VecDeque<String>>>,
    #[allow(dead_code)]
    event_tx: mpsc::Sender<Event>,
    current_node_id: Arc<RwLock<Option<String>>>,
    current_context: Arc<RwLock<Option<PageContext>>>,
    pattern_solver: NavigationPatternSolver,
}

impl PlannerAgent {
    pub fn new(
        id: String,
        graph: Arc<RwLock<ExplorationGraph>>,
        blackboard: Arc<Blackboard>,
        config: VisionExplorerV2Config,
        event_tx: mpsc::Sender<Event>,
    ) -> Self {
        Self {
            id: id.clone(),
            graph,
            blackboard,
            config,
            frontier: Arc::new(RwLock::new(VecDeque::new())),
            event_tx,
            current_node_id: Arc::new(RwLock::new(None)),
            current_context: Arc::new(RwLock::new(None)),
            pattern_solver: NavigationPatternSolver::new(format!("{}_solver", id)),
        }
    }

    /// Add a new discovered node to the details
    #[allow(dead_code)]
    async fn add_to_frontier(&self, node_fingerprint: String) {
        let mut frontier = self.frontier.write().await;
        if !frontier.contains(&node_fingerprint) {
            frontier.push_back(node_fingerprint);
        }
    }

    /// Decide the next best action based on the graph state
    pub async fn decide_next_step(&self) -> Result<Option<Event>> {
        let current_node_opt = self.current_node_id.read().await.clone();

        // 1. If we have a current node, process it
        if let Some(current_id) = current_node_opt {
            // Read node status and actions first, then release lock
            let node_status = {
                let graph = self.graph.read().await;
                graph.get_node(&current_id).map(|node| node.status.clone())
            };

            // A. Node is Unvisited -> Need Perception (Pure Vision)
            if node_status == Some(ExplorationStatus::Unvisited) {
                let ctx_opt = self.current_context.read().await.clone();

                if let Some(ctx) = ctx_opt {
                    log::info!(
                        "Current node {} is Unvisited. Assigning VisualAnalyst for pure vision analysis.",
                        current_id
                    );

                    // Mark as Visiting to prevent duplicate task assignment
                    {
                        let mut graph_w = self.graph.write().await;
                        if let Some(node_mut) = graph_w.get_node_mut(&current_id) {
                            node_mut.status = ExplorationStatus::Visiting;
                        }
                    }

                    return Ok(Some(Event::TaskAssigned {
                        agent_id: "visual_analyst".to_string(),
                        task_id: uuid::Uuid::new_v4().to_string(),
                        target_node_id: current_id,
                        payload: Some(serde_json::to_value(&ctx)?),
                    }));
                } else {
                    log::warn!("Node {} is Unvisited but no Context available!", current_id);
                }
            }

            // B. Node is Analyzed -> Need Action
            if node_status == Some(ExplorationStatus::Analyzed) {
                let mut graph = self.graph.write().await;
                if let Some(node) = graph.get_node_mut(&current_id) {
                    if !node.possible_actions.is_empty() {
                        // === AUTH PRIORITY & SAFETY FILTERING ===
                        let is_authenticated = self.blackboard.is_authenticated().await;
                        
                        // 1. Check for high-priority login actions
                        let priority_idx = node.possible_actions.iter().position(|a| {
                            let desc = a.description.to_lowercase();
                            let is_login_action = desc.contains("login") || desc.contains("sign in") || a.action_type == "fill_form";
                            is_login_action && !is_authenticated
                        });

                        let action_idx = if let Some(idx) = priority_idx {
                            log::info!("PRIORITIZING AUTH ACTION: {}", node.possible_actions[idx].description);
                            idx
                        } else {
                            // 2. Filter out dangerous actions if authenticated (to prevent logout loops)
                            let mut filtered_idx = None;
                            for (idx, a) in node.possible_actions.iter().enumerate() {
                                let desc = a.description.to_lowercase();
                                let is_logout = desc.contains("logout") || desc.contains("sign out") || desc.contains("exit");
                                if is_authenticated && is_logout {
                                    log::warn!("SKIPPING DANGEROUS LOGOUT ACTION: {}", a.description);
                                    continue;
                                }
                                filtered_idx = Some(idx);
                                break;
                            }
                            filtered_idx.unwrap_or(0) // Default to first if none filtered or all skipped
                        };

                        if action_idx < node.possible_actions.len() {
                            let action = node.possible_actions.remove(action_idx);
                            log::info!(
                                "Decided to take action: {} (Type: {}, Selector: {}, Coords: {:?})",
                                action.description,
                                action.action_type,
                                action.selector,
                                (action.x, action.y)
                            );

                            return Ok(Some(Event::TaskAssigned {
                                agent_id: "navigator_1".to_string(),
                                task_id: uuid::Uuid::new_v4().to_string(),
                                target_node_id: current_id,
                                payload: Some(serde_json::to_value(&action)?),
                            }));
                        }
                    }
                    
                    log::info!(
                        "No actions available on node {}, marking as Visited.",
                        current_id
                    );
                    node.status = ExplorationStatus::Visited;
                }
            }
        }

        // 2. If no current node processing needed, check frontier (Backtracking/Jumping)
        let mut frontier = self.frontier.write().await;
        while let Some(next_node_fingerprint) = frontier.pop_front() {
            // For V2 MVP, we only support exploring if we are already there
            // OR if we can jump (URL).
            let url_opt = {
                let graph = self.graph.read().await;
                if let Some(node) = graph.get_node(&next_node_fingerprint) {
                    if node.status == ExplorationStatus::Unvisited {
                        Some(node.url.clone())
                    } else {
                        None
                    }
                } else {
                    None
                }
            };
            if let Some(url) = url_opt {
                // Navigate to it
                return Ok(Some(Event::TaskAssigned {
                    agent_id: "navigator_1".to_string(),
                    task_id: uuid::Uuid::new_v4().to_string(),
                    target_node_id: url,
                    payload: None,
                }));
            }
        }

        Ok(None) // No more tasks
    }
}

#[async_trait]
impl Agent for PlannerAgent {
    fn id(&self) -> String {
        self.id.clone()
    }

    async fn handle_event(&self, event: &Event) -> Result<()> {
        match event {
            Event::TaskCompleted {
                agent_id, result, ..
            } => {
                // If the task failed, we need to reset the node status to prevent hanging
                if !result.success {
                    let mut graph = self.graph.write().await;
                    let current_opt = self.current_node_id.read().await.clone();
                    if let Some(current_id) = current_opt {
                        if let Some(node) = graph.get_node_mut(&current_id) {
                            log::warn!(
                                "Task failed for node {}, resetting to Unvisited",
                                current_id
                            );
                            node.status = ExplorationStatus::Unvisited;
                        }
                    }
                    return Ok(());
                }

                // If Navigator finished, we have a new Context -> New Node
                if agent_id.contains("navigator") {
                    if let Some(data) = &result.data {
                        if let Ok(context) = serde_json::from_value::<PageContext>(data.clone()) {
                            let was_authenticated = self.blackboard.is_authenticated().await;
                            let old_url = self.current_context.read().await.as_ref().map(|c| c.url.clone());
                            
                            // 1. Detect Auth Status Transitions
                            if let Some(old_ctx) = self.current_context.read().await.as_ref() {
                                // Transition: Login -> Dashboard
                                if AuthAgent::detect_login_success(old_ctx, &context) {
                                    log::info!(" Planner: Login SUCCESS detected!");
                                    self.blackboard.set_authenticated(true).await;
                                }
                                
                                // Transition: LoggedIn -> LoggedOut (Dangerous)
                                if was_authenticated && AuthAgent::is_login_page(&context) && !AuthAgent::is_login_page(old_ctx) {
                                    log::warn!(" Planner: AUTHENTICATION LOST (Logout detected)!");
                                    self.blackboard.set_authenticated(false).await;
                                }
                            }

                            let fingerprint = context.fingerprint(self.blackboard.is_authenticated().await);

                            let mut graph = self.graph.write().await;
                            
                            // 2. Clear old actions if URL changed (page navigation occurred)
                            if let (Some(old), new) = (old_url.as_ref(), &context.url) {
                                if old != new {
                                    log::info!(" Planner: URL changed from '{}' to '{}', clearing old actions queue", old, new);
                                    // Clear actions from the old node
                                    if let Some(old_node_id) = self.current_node_id.read().await.as_ref() {
                                        if let Some(old_node) = graph.get_node_mut(old_node_id) {
                                            old_node.possible_actions.clear();
                                            log::debug!(" Planner: Cleared {} pending actions from old page", old_node.possible_actions.len());
                                        }
                                    }
                                }
                            }
                            
                            // Check if node exists
                            if !graph.has_state(&fingerprint) {
                                let node = PageStateNode {
                                    fingerprint: fingerprint.clone(),
                                    url: context.url.clone(),
                                    title: context.title.clone(),
                                    status: ExplorationStatus::Unvisited,
                                    depth: 0,
                                    page_type: None,
                                    possible_actions: vec![],
                                };
                                graph.add_node(node);
                                // Add to frontier
                                let mut frontier = self.frontier.write().await;
                                frontier.push_back(fingerprint.clone());
                            }

                            // Update Current Node
                            let mut current = self.current_node_id.write().await;
                            *current = Some(fingerprint.clone());

                            // Update Current Context
                            let mut ctx_lock = self.current_context.write().await;
                            *ctx_lock = Some(context.clone());
                        }
                    }
                }
                // If Analyst finished, we have PerceptionResult
                else if agent_id.contains("analyst") {
                    if let Some(data) = &result.data {
                        if let Ok(perception) =
                            serde_json::from_value::<PerceptionResult>(data.clone())
                        {
                            // === INTELLIGENCE AUGMENTATION ===
                            // Use PatternSolver to enhance actions (e.g., detect sidebars)
                            let enhanced_actions = self
                                .pattern_solver
                                .detect_patterns(&perception.suggested_actions);

                            let mut graph = self.graph.write().await;
                            let current_opt = self.current_node_id.read().await.clone();
                            if let Some(current_id) = current_opt {
                                if let Some(node) = graph.get_node_mut(&current_id) {
                                    node.possible_actions = enhanced_actions; // Use enhanced!
                                    node.status = ExplorationStatus::Analyzed;
                                }
                            }
                        }
                    }
                }
                Ok(())
            }
            _ => Ok(()),
        }
    }
}
