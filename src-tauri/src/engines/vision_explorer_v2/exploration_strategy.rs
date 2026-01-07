//! Exploration Strategy - Graph traversal algorithms for systematic exploration
//!
//! This module provides different strategies for exploring web applications,
//! including breadth-first, depth-first, and priority-based approaches.

use crate::engines::vision_explorer_v2::graph::{ExplorationGraph, PageStateNode};
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet, VecDeque};
use tracing::debug;

/// Priority score for a node (higher = more important)
pub type Priority = f32;

/// Exploration strategy trait
pub trait ExplorationStrategy: Send + Sync {
    /// Get the next node to visit
    fn next_node(&mut self, graph: &ExplorationGraph, current: Option<&str>) -> Option<String>;

    /// Check if a node should be visited
    fn should_visit(&self, node: &PageStateNode) -> bool;

    /// Update strategy state after visiting a node
    fn on_node_visited(&mut self, node_id: &str, success: bool);

    /// Reset strategy state
    fn reset(&mut self);

    /// Get strategy name for debugging
    fn name(&self) -> &'static str;
}

/// Breadth-First Search strategy - explores level by level
pub struct BFSStrategy {
    visited: HashSet<String>,
    queue: VecDeque<String>,
    max_depth: u32,
}

impl BFSStrategy {
    pub fn new(max_depth: u32) -> Self {
        Self {
            visited: HashSet::new(),
            queue: VecDeque::new(),
            max_depth,
        }
    }
}

impl ExplorationStrategy for BFSStrategy {
    fn next_node(&mut self, graph: &ExplorationGraph, current: Option<&str>) -> Option<String> {
        // If we have a current node, add its unvisited neighbors to queue
        if let Some(current_id) = current {
            if let Some(current_node) = graph.get_node(current_id) {
                if current_node.depth < self.max_depth {
                    for edge in graph.get_outgoing_edges(current_id) {
                        if !self.visited.contains(&edge.target_node_id) 
                            && !self.queue.contains(&edge.target_node_id) {
                            self.queue.push_back(edge.target_node_id.clone());
                        }
                    }
                }
            }
        }

        // Return next node from queue
        while let Some(node_id) = self.queue.pop_front() {
            if !self.visited.contains(&node_id) {
                if let Some(node) = graph.get_node(&node_id) {
                    if self.should_visit(node) {
                        return Some(node_id);
                    }
                }
            }
        }

        None
    }

    fn should_visit(&self, node: &PageStateNode) -> bool {
        !self.visited.contains(&node.fingerprint) && node.depth <= self.max_depth
    }

    fn on_node_visited(&mut self, node_id: &str, _success: bool) {
        self.visited.insert(node_id.to_string());
        debug!("BFS: Visited node {} (total visited: {})", node_id, self.visited.len());
    }

    fn reset(&mut self) {
        self.visited.clear();
        self.queue.clear();
    }

    fn name(&self) -> &'static str {
        "BFS"
    }
}

/// Depth-First Search strategy - explores as deep as possible
pub struct DFSStrategy {
    visited: HashSet<String>,
    stack: Vec<String>,
    max_depth: u32,
}

impl DFSStrategy {
    pub fn new(max_depth: u32) -> Self {
        Self {
            visited: HashSet::new(),
            stack: Vec::new(),
            max_depth,
        }
    }
}

impl ExplorationStrategy for DFSStrategy {
    fn next_node(&mut self, graph: &ExplorationGraph, current: Option<&str>) -> Option<String> {
        // If we have a current node, add its unvisited neighbors to stack
        if let Some(current_id) = current {
            if let Some(current_node) = graph.get_node(current_id) {
                if current_node.depth < self.max_depth {
                    let mut neighbors = Vec::new();
                    for edge in graph.get_outgoing_edges(current_id) {
                        if !self.visited.contains(&edge.target_node_id) 
                            && !self.stack.contains(&edge.target_node_id) {
                            neighbors.push(edge.target_node_id.clone());
                        }
                    }
                    // Add in reverse order so we explore in original order
                    neighbors.reverse();
                    self.stack.extend(neighbors);
                }
            }
        }

        // Return next node from stack
        while let Some(node_id) = self.stack.pop() {
            if !self.visited.contains(&node_id) {
                if let Some(node) = graph.get_node(&node_id) {
                    if self.should_visit(node) {
                        return Some(node_id);
                    }
                }
            }
        }

        None
    }

    fn should_visit(&self, node: &PageStateNode) -> bool {
        !self.visited.contains(&node.fingerprint) && node.depth <= self.max_depth
    }

    fn on_node_visited(&mut self, node_id: &str, _success: bool) {
        self.visited.insert(node_id.to_string());
        debug!("DFS: Visited node {} (total visited: {})", node_id, self.visited.len());
    }

    fn reset(&mut self) {
        self.visited.clear();
        self.stack.clear();
    }

    fn name(&self) -> &'static str {
        "DFS"
    }
}

/// Priority-based strategy - visits nodes based on calculated importance
pub struct PriorityStrategy {
    visited: HashSet<String>,
    priorities: HashMap<String, Priority>,
    max_depth: u32,
    priority_calculator: Box<dyn PriorityCalculator>,
}

impl PriorityStrategy {
    pub fn new(max_depth: u32, calculator: Box<dyn PriorityCalculator>) -> Self {
        Self {
            visited: HashSet::new(),
            priorities: HashMap::new(),
            max_depth,
            priority_calculator: calculator,
        }
    }
}

impl ExplorationStrategy for PriorityStrategy {
    fn next_node(&mut self, graph: &ExplorationGraph, _current: Option<&str>) -> Option<String> {
        // Update priorities for all unvisited nodes
        for node in graph.get_all_nodes() {
            if !self.visited.contains(&node.fingerprint) && self.should_visit(node) {
                let priority = self.priority_calculator.calculate_priority(node, graph);
                self.priorities.insert(node.fingerprint.clone(), priority);
            }
        }

        // Find highest priority unvisited node
        let mut best_node: Option<String> = None;
        let mut best_priority = f32::NEG_INFINITY;

        for (node_id, priority) in &self.priorities {
            if !self.visited.contains(node_id) && *priority > best_priority {
                if let Some(node) = graph.get_node(node_id) {
                    if self.should_visit(node) {
                        best_node = Some(node_id.clone());
                        best_priority = *priority;
                    }
                }
            }
        }

        best_node
    }

    fn should_visit(&self, node: &PageStateNode) -> bool {
        !self.visited.contains(&node.fingerprint) && node.depth <= self.max_depth
    }

    fn on_node_visited(&mut self, node_id: &str, success: bool) {
        self.visited.insert(node_id.to_string());
        self.priorities.remove(node_id);
        debug!(
            "Priority: Visited node {} (success: {}, total visited: {})",
            node_id, success, self.visited.len()
        );
    }

    fn reset(&mut self) {
        self.visited.clear();
        self.priorities.clear();
    }

    fn name(&self) -> &'static str {
        "Priority"
    }
}

/// Priority calculation trait
pub trait PriorityCalculator: Send + Sync {
    fn calculate_priority(&self, node: &PageStateNode, graph: &ExplorationGraph) -> Priority;
}

/// Default priority calculator based on URL patterns and page types
pub struct DefaultPriorityCalculator;

impl PriorityCalculator for DefaultPriorityCalculator {
    fn calculate_priority(&self, node: &PageStateNode, _graph: &ExplorationGraph) -> Priority {
        let mut priority = 1.0;

        // Higher priority for login/auth pages
        if node.url.to_lowercase().contains("login") 
            || node.url.to_lowercase().contains("auth") 
            || node.url.to_lowercase().contains("signin") {
            priority += 10.0;
        }

        // Higher priority for admin/dashboard pages
        if node.url.to_lowercase().contains("admin") 
            || node.url.to_lowercase().contains("dashboard") 
            || node.url.to_lowercase().contains("panel") {
            priority += 8.0;
        }

        // Higher priority for API endpoints
        if node.url.to_lowercase().contains("api") 
            || node.url.to_lowercase().contains("/v1/") 
            || node.url.to_lowercase().contains("/v2/") {
            priority += 15.0;
        }

        // Lower priority for static resources
        if node.url.ends_with(".css") 
            || node.url.ends_with(".js") 
            || node.url.ends_with(".png") 
            || node.url.ends_with(".jpg") 
            || node.url.ends_with(".gif") {
            priority -= 5.0;
        }

        // Lower priority for deeper nodes (prefer breadth)
        priority -= node.depth as f32 * 0.5;

        // Higher priority for pages with forms (based on possible actions)
        let has_form_actions = node.possible_actions.iter()
            .any(|action| action.action_type == "fill_form" || action.action_type == "submit");
        if has_form_actions {
            priority += 5.0;
        }

        priority
    }
}

/// Smart strategy that adapts based on what's been discovered
pub struct AdaptiveStrategy {
    current_strategy: Box<dyn ExplorationStrategy>,
    bfs_strategy: BFSStrategy,
    priority_strategy: PriorityStrategy,
    nodes_visited: u32,
    switch_threshold: u32,
}

impl AdaptiveStrategy {
    pub fn new(max_depth: u32) -> Self {
        let bfs = BFSStrategy::new(max_depth);
        let priority = PriorityStrategy::new(max_depth, Box::new(DefaultPriorityCalculator));
        
        Self {
            current_strategy: Box::new(BFSStrategy::new(max_depth)),
            bfs_strategy: bfs,
            priority_strategy: priority,
            nodes_visited: 0,
            switch_threshold: 10, // Switch to priority after 10 nodes
        }
    }
}

impl ExplorationStrategy for AdaptiveStrategy {
    fn next_node(&mut self, graph: &ExplorationGraph, current: Option<&str>) -> Option<String> {
        // Switch strategy if threshold reached
        if self.nodes_visited >= self.switch_threshold && self.current_strategy.name() == "BFS" {
            debug!("Adaptive: Switching from BFS to Priority strategy after {} nodes", self.nodes_visited);
            self.current_strategy = Box::new(PriorityStrategy::new(
                self.bfs_strategy.max_depth,
                Box::new(DefaultPriorityCalculator)
            ));
        }

        self.current_strategy.next_node(graph, current)
    }

    fn should_visit(&self, node: &PageStateNode) -> bool {
        self.current_strategy.should_visit(node)
    }

    fn on_node_visited(&mut self, node_id: &str, success: bool) {
        self.nodes_visited += 1;
        self.current_strategy.on_node_visited(node_id, success);
    }

    fn reset(&mut self) {
        self.nodes_visited = 0;
        self.current_strategy.reset();
    }

    fn name(&self) -> &'static str {
        "Adaptive"
    }
}

/// Strategy configuration for easy switching
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum StrategyConfig {
    BFS { max_depth: u32 },
    DFS { max_depth: u32 },
    Priority { max_depth: u32 },
    Adaptive { max_depth: u32 },
}

impl Default for StrategyConfig {
    fn default() -> Self {
        Self::Adaptive { max_depth: 5 }
    }
}

/// Factory function to create strategies from config
pub fn create_strategy(config: StrategyConfig) -> Box<dyn ExplorationStrategy> {
    match config {
        StrategyConfig::BFS { max_depth } => Box::new(BFSStrategy::new(max_depth)),
        StrategyConfig::DFS { max_depth } => Box::new(DFSStrategy::new(max_depth)),
        StrategyConfig::Priority { max_depth } => Box::new(PriorityStrategy::new(
            max_depth,
            Box::new(DefaultPriorityCalculator),
        )),
        StrategyConfig::Adaptive { max_depth } => Box::new(AdaptiveStrategy::new(max_depth)),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::engines::vision_explorer_v2::graph::GraphEdge;

    fn create_test_graph() -> ExplorationGraph {
        let mut graph = ExplorationGraph::new();
        
        // Add nodes
        graph.add_node("root".to_string(), "https://example.com".to_string(), "Home".to_string(), 0);
        graph.add_node("login".to_string(), "https://example.com/login".to_string(), "Login".to_string(), 1);
        graph.add_node("admin".to_string(), "https://example.com/admin".to_string(), "Admin".to_string(), 1);
        graph.add_node("api".to_string(), "https://example.com/api/users".to_string(), "API".to_string(), 2);
        
        // Add edges
        graph.add_edge("root".to_string(), "login".to_string(), "click_login".to_string());
        graph.add_edge("root".to_string(), "admin".to_string(), "click_admin".to_string());
        graph.add_edge("login".to_string(), "api".to_string(), "authenticated".to_string());
        
        graph
    }

    #[test]
    fn test_bfs_strategy() {
        let mut strategy = BFSStrategy::new(5);
        let graph = create_test_graph();
        
        // Should start with root's neighbors
        let next = strategy.next_node(&graph, Some("root"));
        assert!(next.is_some());
        
        let node_id = next.unwrap();
        strategy.on_node_visited(&node_id, true);
        
        // Should not revisit the same node
        assert!(!strategy.should_visit(graph.get_node(&node_id).unwrap()));
    }

    #[test]
    fn test_priority_calculator() {
        let calculator = DefaultPriorityCalculator;
        let graph = create_test_graph();
        
        let login_node = graph.get_node("login").unwrap();
        let api_node = graph.get_node("api").unwrap();
        let root_node = graph.get_node("root").unwrap();
        
        let login_priority = calculator.calculate_priority(login_node, &graph);
        let api_priority = calculator.calculate_priority(api_node, &graph);
        let root_priority = calculator.calculate_priority(root_node, &graph);
        
        // API should have highest priority
        assert!(api_priority > login_priority);
        // Login should have higher priority than root
        assert!(login_priority > root_priority);
    }

    #[test]
    fn test_strategy_factory() {
        let bfs = create_strategy(StrategyConfig::BFS { max_depth: 3 });
        assert_eq!(bfs.name(), "BFS");
        
        let dfs = create_strategy(StrategyConfig::DFS { max_depth: 3 });
        assert_eq!(dfs.name(), "DFS");
        
        let priority = create_strategy(StrategyConfig::Priority { max_depth: 3 });
        assert_eq!(priority.name(), "Priority");
        
        let adaptive = create_strategy(StrategyConfig::Adaptive { max_depth: 3 });
        assert_eq!(adaptive.name(), "Adaptive");
    }
}