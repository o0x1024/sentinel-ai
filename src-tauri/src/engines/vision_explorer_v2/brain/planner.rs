use crate::engines::vision_explorer_v2::brain::pattern_solver::NavigationPatternSolver;
use crate::engines::vision_explorer_v2::core::{
    Agent, Event, PageContext, PerceptionResult, SuggestedAction,
};
use crate::engines::vision_explorer_v2::graph::{
    ExplorationGraph, ExplorationStatus, PageStateNode,
};
use crate::engines::vision_explorer_v2::VisionExplorerV2Config;
use anyhow::{Context, Result};
use async_trait::async_trait;
use std::collections::VecDeque;
use std::sync::Arc;
use tokio::sync::{mpsc, RwLock};

/// The Strategist. Manages the exploration frontier and assigns tasks.
pub struct PlannerAgent {
    id: String,
    graph: Arc<RwLock<ExplorationGraph>>,
    config: VisionExplorerV2Config,
    frontier: Arc<RwLock<VecDeque<String>>>,
    event_tx: mpsc::Sender<Event>,
    current_node_id: Arc<RwLock<Option<String>>>,
    current_context: Arc<RwLock<Option<PageContext>>>,
    pattern_solver: NavigationPatternSolver,
}

impl PlannerAgent {
    pub fn new(
        id: String,
        graph: Arc<RwLock<ExplorationGraph>>,
        config: VisionExplorerV2Config,
        event_tx: mpsc::Sender<Event>,
    ) -> Self {
        Self {
            id: id.clone(),
            graph,
            config,
            frontier: Arc::new(RwLock::new(VecDeque::new())),
            event_tx,
            current_node_id: Arc::new(RwLock::new(None)),
            current_context: Arc::new(RwLock::new(None)),
            pattern_solver: NavigationPatternSolver::new(format!("{}_solver", id)),
        }
    }

    /// Add a new discovered node to the details
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
            let (node_status, first_action) = {
                let graph = self.graph.read().await;
                if let Some(node) = graph.get_node(&current_id) {
                    (
                        Some(node.status.clone()),
                        node.possible_actions.first().cloned(),
                    )
                } else {
                    (None, None)
                }
            };

            // A. Node is Unvisited -> Need Perception
            if node_status == Some(ExplorationStatus::Unvisited) {
                let ctx_opt = self.current_context.read().await.clone();

                if let Some(ctx) = ctx_opt {
                    // === HYBRID STRATEGY: Fast-Pass vs Deep-Dive ===
                    // Heuristic: If DOM is very small (< 1KB) but screenshot exists,
                    // or if we detect Canvas, assign VisualAnalyst.
                    // Otherwise, default to StructuralAnalyst (Fast).

                    let is_complex_visual = ctx.dom_snapshot.len() < 1000
                        || ctx.dom_snapshot.contains("<canvas")
                        || ctx.dom_snapshot.contains("<svg");

                    let target_agent = if is_complex_visual {
                        log::info!(
                            "Current node {} seems VISUALLY COMPLEX. Assigning VisualAnalyst.",
                            current_id
                        );
                        "visual_analyst"
                    } else {
                        log::info!(
                            "Current node {} seems Standard. Assigning StructuralAnalyst.",
                            current_id
                        );
                        "structural_analyst"
                    };

                    // Mark as Visiting to prevent duplicate task assignment
                    {
                        let mut graph_w = self.graph.write().await;
                        if let Some(node_mut) = graph_w.get_node_mut(&current_id) {
                            node_mut.status = ExplorationStatus::Visiting;
                        }
                    }

                    return Ok(Some(Event::TaskAssigned {
                        agent_id: target_agent.to_string(),
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
                if let Some(action) = first_action {
                    log::info!(
                        "Decided to take action: {} on {}",
                        action.action_type,
                        action.selector
                    );
                    return Ok(Some(Event::TaskAssigned {
                        agent_id: "navigator_1".to_string(),
                        task_id: uuid::Uuid::new_v4().to_string(),
                        target_node_id: current_id,
                        payload: Some(serde_json::to_value(&action)?),
                    }));
                } else {
                    log::info!("No actions available on node {}.", current_id);
                    // TODO: Mark as Visited/Exhausted and Backtrack
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
                // If Navigator finished, we have a new Context -> New Node
                if agent_id.contains("navigator") {
                    if let Some(data) = &result.data {
                        if let Ok(context) = serde_json::from_value::<PageContext>(data.clone()) {
                            let fingerprint = context.fingerprint();

                            let mut graph = self.graph.write().await;
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
