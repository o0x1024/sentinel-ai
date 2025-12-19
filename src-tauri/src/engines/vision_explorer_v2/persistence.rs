//! Persistence Layer - Save and restore exploration state
//!
//! Enables:
//! - Crash recovery: Resume exploration from last known state
//! - Session management: Save/load exploration sessions
//! - Incremental exploration: Continue from previous runs

use crate::engines::vision_explorer_v2::blackboard::BlackboardData;
use crate::engines::vision_explorer_v2::graph::{ActionEdge, ExplorationGraph, PageStateNode};
use anyhow::{Context, Result};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::path::Path;

/// A serializable snapshot of the exploration state
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExplorationSnapshot {
    /// Unique session ID
    pub session_id: String,

    /// When this snapshot was created
    pub created_at: DateTime<Utc>,

    /// When exploration started
    pub started_at: DateTime<Utc>,

    /// Target URL
    pub target_url: String,

    /// All discovered nodes
    pub nodes: Vec<PageStateNode>,

    /// All discovered edges (as tuples: from_fingerprint, to_fingerprint, edge)
    pub edges: Vec<(String, String, ActionEdge)>,

    /// Blackboard state
    pub blackboard: BlackboardData,

    /// Current exploration frontier (fingerprints of nodes to explore)
    pub frontier: Vec<String>,

    /// Current node being explored (if any)
    pub current_node: Option<String>,

    /// Total steps taken
    pub steps_taken: u32,

    /// Exploration statistics
    pub stats: ExplorationStats,
}

/// Statistics about the exploration
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ExplorationStats {
    /// Total pages visited
    pub pages_visited: u32,

    /// Total actions performed
    pub actions_performed: u32,

    /// Total forms filled
    pub forms_filled: u32,

    /// Total errors encountered
    pub errors_encountered: u32,

    /// Total APIs discovered
    pub apis_discovered: u32,

    /// Total secrets found
    pub secrets_found: u32,
}

impl ExplorationSnapshot {
    /// Create a new snapshot with the given session ID and target
    pub fn new(session_id: String, target_url: String) -> Self {
        let now = Utc::now();
        Self {
            session_id,
            created_at: now,
            started_at: now,
            target_url,
            nodes: vec![],
            edges: vec![],
            blackboard: BlackboardData::default(),
            frontier: vec![],
            current_node: None,
            steps_taken: 0,
            stats: ExplorationStats::default(),
        }
    }

    /// Save snapshot to a JSON file
    pub fn save_to_file(&self, path: &Path) -> Result<()> {
        let json = serde_json::to_string_pretty(self).context("Failed to serialize snapshot")?;
        std::fs::write(path, json).context("Failed to write snapshot file")?;
        Ok(())
    }

    /// Load snapshot from a JSON file
    pub fn load_from_file(path: &Path) -> Result<Self> {
        let json = std::fs::read_to_string(path).context("Failed to read snapshot file")?;
        let snapshot: Self =
            serde_json::from_str(&json).context("Failed to deserialize snapshot")?;
        Ok(snapshot)
    }

    /// Save snapshot to a JSON string
    pub fn to_json(&self) -> Result<String> {
        serde_json::to_string_pretty(self).context("Failed to serialize snapshot")
    }

    /// Load snapshot from a JSON string
    pub fn from_json(json: &str) -> Result<Self> {
        serde_json::from_str(json).context("Failed to deserialize snapshot")
    }

    /// Update the snapshot timestamp
    pub fn touch(&mut self) {
        self.created_at = Utc::now();
    }
}

/// Manager for exploration persistence
pub struct PersistenceManager {
    /// Directory for storing snapshots
    storage_dir: std::path::PathBuf,
    /// Current session ID
    session_id: String,
    /// Auto-save interval (in steps)
    auto_save_interval: u32,
    /// Steps since last save
    steps_since_save: u32,
}

impl PersistenceManager {
    pub fn new(storage_dir: impl Into<std::path::PathBuf>, session_id: String) -> Self {
        let storage_dir = storage_dir.into();
        // Ensure directory exists
        let _ = std::fs::create_dir_all(&storage_dir);

        Self {
            storage_dir,
            session_id,
            auto_save_interval: 10, // Save every 10 steps
            steps_since_save: 0,
        }
    }

    /// Get the path for the current session snapshot
    fn snapshot_path(&self) -> std::path::PathBuf {
        self.storage_dir.join(format!("{}.json", self.session_id))
    }

    /// Save a snapshot
    pub fn save(&self, snapshot: &ExplorationSnapshot) -> Result<()> {
        snapshot.save_to_file(&self.snapshot_path())
    }

    /// Load the current session's snapshot
    pub fn load(&self) -> Result<Option<ExplorationSnapshot>> {
        let path = self.snapshot_path();
        if path.exists() {
            Ok(Some(ExplorationSnapshot::load_from_file(&path)?))
        } else {
            Ok(None)
        }
    }

    /// Check if a snapshot exists for this session
    pub fn has_snapshot(&self) -> bool {
        self.snapshot_path().exists()
    }

    /// Record a step and auto-save if needed
    pub fn record_step(&mut self, snapshot: &ExplorationSnapshot) -> Result<bool> {
        self.steps_since_save += 1;
        if self.steps_since_save >= self.auto_save_interval {
            self.save(snapshot)?;
            self.steps_since_save = 0;
            Ok(true) // Saved
        } else {
            Ok(false) // Not saved
        }
    }

    /// List all available sessions
    pub fn list_sessions(&self) -> Result<Vec<SessionInfo>> {
        let mut sessions = vec![];

        for entry in std::fs::read_dir(&self.storage_dir)? {
            let entry = entry?;
            let path = entry.path();
            if path.extension().map(|e| e == "json").unwrap_or(false) {
                if let Ok(snapshot) = ExplorationSnapshot::load_from_file(&path) {
                    sessions.push(SessionInfo {
                        session_id: snapshot.session_id,
                        target_url: snapshot.target_url,
                        created_at: snapshot.created_at,
                        started_at: snapshot.started_at,
                        steps_taken: snapshot.steps_taken,
                        nodes_count: snapshot.nodes.len() as u32,
                    });
                }
            }
        }

        // Sort by creation time, newest first
        sessions.sort_by(|a, b| b.created_at.cmp(&a.created_at));

        Ok(sessions)
    }

    /// Delete a session's snapshot
    pub fn delete_session(&self, session_id: &str) -> Result<()> {
        let path = self.storage_dir.join(format!("{}.json", session_id));
        if path.exists() {
            std::fs::remove_file(path)?;
        }
        Ok(())
    }

    /// Set auto-save interval
    pub fn set_auto_save_interval(&mut self, steps: u32) {
        self.auto_save_interval = steps;
    }
}

/// Information about a saved session
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SessionInfo {
    pub session_id: String,
    pub target_url: String,
    pub created_at: DateTime<Utc>,
    pub started_at: DateTime<Utc>,
    pub steps_taken: u32,
    pub nodes_count: u32,
}

/// Helper to convert ExplorationGraph to/from snapshot format
pub mod graph_serialization {
    use super::*;

    /// Extract nodes and edges from a graph for serialization
    pub fn graph_to_snapshot_data(
        graph: &ExplorationGraph,
    ) -> (Vec<PageStateNode>, Vec<(String, String, ActionEdge)>) {
        // Note: This requires access to the internal graph structure
        // For now, we'll just return empty collections
        // In a real implementation, ExplorationGraph would need methods to expose this data
        (vec![], vec![])
    }

    /// Rebuild a graph from snapshot data
    pub fn snapshot_data_to_graph(
        nodes: Vec<PageStateNode>,
        edges: Vec<(String, String, ActionEdge)>,
    ) -> ExplorationGraph {
        let mut graph = ExplorationGraph::new();

        // Add all nodes first
        for node in nodes {
            graph.add_node(node);
        }

        // Then add edges
        for (from_fp, to_fp, edge) in edges {
            if let (Some(from_idx), Some(to_idx)) =
                (graph.get_node_index(&from_fp), graph.get_node_index(&to_fp))
            {
                graph.add_edge(from_idx, to_idx, edge);
            }
        }

        graph
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[test]
    fn test_snapshot_serialization() {
        let snapshot = ExplorationSnapshot::new(
            "test-session".to_string(),
            "https://example.com".to_string(),
        );

        let json = snapshot.to_json().unwrap();
        let restored = ExplorationSnapshot::from_json(&json).unwrap();

        assert_eq!(snapshot.session_id, restored.session_id);
        assert_eq!(snapshot.target_url, restored.target_url);
    }

    #[test]
    fn test_file_persistence() {
        let dir = tempdir().unwrap();
        let snapshot = ExplorationSnapshot::new(
            "test-session".to_string(),
            "https://example.com".to_string(),
        );

        let path = dir.path().join("test.json");
        snapshot.save_to_file(&path).unwrap();

        let loaded = ExplorationSnapshot::load_from_file(&path).unwrap();
        assert_eq!(snapshot.session_id, loaded.session_id);
    }
}
