use crate::engines::vision_explorer_v2::core::SuggestedAction;
use petgraph::graph::{DiGraph, NodeIndex};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// The Global Exploration Graph
/// Maps the application structure as a directed graph of States (Nodes) and Actions (Edges).
#[derive(Debug, Clone)]
pub struct ExplorationGraph {
    /// The underlying graph structure
    graph: DiGraph<PageStateNode, ActionEdge>,
    /// Map of state fingerprint to node index for quick lookups
    node_map: HashMap<String, NodeIndex>,
}

impl Default for ExplorationGraph {
    fn default() -> Self {
        Self::new()
    }
}

impl ExplorationGraph {
    pub fn new() -> Self {
        Self {
            graph: DiGraph::new(),
            node_map: HashMap::new(),
        }
    }

    /// Add a new node to the graph if it doesn't exist
    pub fn add_node(&mut self, state: PageStateNode) -> NodeIndex {
        if let Some(&idx) = self.node_map.get(&state.fingerprint) {
            return idx;
        }

        let fingerprint = state.fingerprint.clone();
        let idx = self.graph.add_node(state);
        self.node_map.insert(fingerprint, idx);
        idx
    }

    /// Add an edge between two nodes
    pub fn add_edge(&mut self, from: NodeIndex, to: NodeIndex, action: ActionEdge) {
        self.graph.add_edge(from, to, action);
    }

    /// Check if a state already exists
    pub fn has_state(&self, fingerprint: &str) -> bool {
        self.node_map.contains_key(fingerprint)
    }

    /// Get a node by fingerprint
    pub fn get_node(&self, fingerprint: &str) -> Option<&PageStateNode> {
        self.node_map.get(fingerprint).map(|&idx| &self.graph[idx])
    }

    pub fn get_node_mut(&mut self, fingerprint: &str) -> Option<&mut PageStateNode> {
        if let Some(&idx) = self.node_map.get(fingerprint) {
            Some(&mut self.graph[idx])
        } else {
            None
        }
    }

    pub fn get_node_index(&self, fingerprint: &str) -> Option<NodeIndex> {
        self.node_map.get(fingerprint).cloned()
    }

    /// Get all outgoing edges from a node by fingerprint
    pub fn get_outgoing_edges(&self, fingerprint: &str) -> Vec<ActionEdge> {
        if let Some(&idx) = self.node_map.get(fingerprint) {
            self.graph
                .edges(idx)
                .map(|edge| edge.weight().clone())
                .collect()
        } else {
            vec![]
        }
    }

    /// Get all nodes in the graph
    pub fn get_all_nodes(&self) -> Vec<&PageStateNode> {
        self.graph.node_weights().collect()
    }
}

/// Representative of a unique Page State (Node)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PageStateNode {
    /// Unique hash of the state (URL + DOM structure hash)
    pub fingerprint: String,
    /// The URL of this state
    pub url: String,
    /// Page title
    pub title: String,
    /// Exploration status of this node
    pub status: ExplorationStatus,
    /// Distance from entry point
    pub depth: u32,
    /// Type of page (e.g., Login, Dashboard, 404)
    pub page_type: Option<String>,
    /// Actions discovered on this page
    #[serde(default)]
    pub possible_actions: Vec<SuggestedAction>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum ExplorationStatus {
    Unvisited,
    Visiting,
    Analyzed,
    Visited,
    Failed,
}

/// Representative of an Action (Edge)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ActionEdge {
    /// Type of action (Click, Submit, Navigate)
    pub action_type: String,
    /// Selector target
    pub selector: Option<String>,
    /// Description of the action
    pub description: String,
    /// Was this action successful?
    pub success: bool,
    /// Target node fingerprint (for graph traversal)
    pub target_node_id: String,
}
