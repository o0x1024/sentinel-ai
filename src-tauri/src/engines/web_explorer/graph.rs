//! Simplified Exploration Graph for ReAct Architecture
//!
//! Records the exploration path as a simple graph structure

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Exploration graph - tracks visited pages and transitions
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExplorationGraph {
    nodes: HashMap<String, GraphNode>,
    edges: Vec<GraphEdge>,
}

/// A node in the exploration graph (represents a page state)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GraphNode {
    pub id: String,
    pub url: String,
    pub title: String,
    pub page_type: String,
    pub depth: u32,
    pub visited_at: u64,
}

/// An edge in the exploration graph (represents a transition)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GraphEdge {
    pub from: String,
    pub to: String,
    pub action: String,
    pub timestamp: u64,
}

impl ExplorationGraph {
    /// Create a new empty graph
    pub fn new() -> Self {
        Self {
            nodes: HashMap::new(),
            edges: Vec::new(),
        }
    }

    /// Add a node to the graph
    pub fn add_node(
        &mut self,
        id: String,
        url: String,
        title: String,
        page_type: String,
        depth: u32,
    ) {
        let node = GraphNode {
            id: id.clone(),
            url,
            title,
            page_type,
            depth,
            visited_at: std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_millis() as u64,
        };
        self.nodes.insert(id, node);
    }

    /// Add an edge between two nodes
    pub fn add_edge(&mut self, from: String, to: String, action: String) {
        let edge = GraphEdge {
            from,
            to,
            action,
            timestamp: std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_millis() as u64,
        };
        self.edges.push(edge);
    }

    /// Check if a node exists
    pub fn has_node(&self, id: &str) -> bool {
        self.nodes.contains_key(id)
    }

    /// Get a node by ID
    pub fn get_node(&self, id: &str) -> Option<&GraphNode> {
        self.nodes.get(id)
    }

    /// Get all nodes
    pub fn get_all_nodes(&self) -> Vec<&GraphNode> {
        self.nodes.values().collect()
    }

    /// Get all edges
    pub fn get_all_edges(&self) -> &[GraphEdge] {
        &self.edges
    }

    /// Get edges from a specific node
    pub fn get_edges_from(&self, node_id: &str) -> Vec<&GraphEdge> {
        self.edges.iter().filter(|e| e.from == node_id).collect()
    }

    /// Get number of nodes
    pub fn node_count(&self) -> usize {
        self.nodes.len()
    }

    /// Get number of edges
    pub fn edge_count(&self) -> usize {
        self.edges.len()
    }

    /// Export graph to JSON for visualization
    pub fn to_json(&self) -> serde_json::Value {
        serde_json::json!({
            "nodes": self.nodes.values().collect::<Vec<_>>(),
            "edges": &self.edges,
            "stats": {
                "node_count": self.node_count(),
                "edge_count": self.edge_count(),
            }
        })
    }
}

impl Default for ExplorationGraph {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_graph_basic() {
        let mut graph = ExplorationGraph::new();
        
        graph.add_node(
            "node1".to_string(),
            "https://example.com".to_string(),
            "Home".to_string(),
            "dashboard".to_string(),
            0,
        );
        
        graph.add_node(
            "node2".to_string(),
            "https://example.com/about".to_string(),
            "About".to_string(),
            "static".to_string(),
            1,
        );
        
        graph.add_edge("node1".to_string(), "node2".to_string(), "click".to_string());
        
        assert_eq!(graph.node_count(), 2);
        assert_eq!(graph.edge_count(), 1);
        assert!(graph.has_node("node1"));
        assert!(graph.has_node("node2"));
    }

    #[test]
    fn test_get_edges_from() {
        let mut graph = ExplorationGraph::new();
        
        graph.add_node("n1".to_string(), "url1".to_string(), "t1".to_string(), "dashboard".to_string(), 0);
        graph.add_node("n2".to_string(), "url2".to_string(), "t2".to_string(), "form".to_string(), 1);
        graph.add_node("n3".to_string(), "url3".to_string(), "t3".to_string(), "list".to_string(), 1);
        
        graph.add_edge("n1".to_string(), "n2".to_string(), "click".to_string());
        graph.add_edge("n1".to_string(), "n3".to_string(), "navigate".to_string());
        
        let edges = graph.get_edges_from("n1");
        assert_eq!(edges.len(), 2);
    }
}
