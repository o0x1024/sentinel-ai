//! Agent Framework - Enhanced trait definitions and lifecycle management
//!
//! This module defines the improved Agent trait with better lifecycle management,
//! state queries, and event handling that returns new events for cascading effects.

use crate::engines::vision_explorer_v2::core::Event;
use anyhow::Result;
use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use std::fmt::Debug;
use std::sync::Arc;

/// Agent execution status
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum AgentStatus {
    /// Agent is not initialized
    Uninitialized,
    /// Agent is initializing
    Initializing,
    /// Agent is idle and ready for tasks
    Idle,
    /// Agent is currently processing a task
    Processing,
    /// Agent is in error state
    Error,
    /// Agent is shutting down
    ShuttingDown,
    /// Agent is shut down
    Shutdown,
}

impl std::fmt::Display for AgentStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{:?}", self)
    }
}

/// Agent metadata for tracking and debugging
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentMetadata {
    /// Unique agent identifier
    pub id: String,
    /// Human-readable agent name
    pub name: String,
    /// Agent description
    pub description: String,
    /// Version of agent implementation
    pub version: String,
    /// Tags for categorization
    pub tags: Vec<String>,
}

/// Agent performance metrics
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct AgentMetrics {
    /// Total events processed
    pub events_processed: u64,
    /// Total tasks completed successfully
    pub tasks_completed: u64,
    /// Total tasks failed
    pub tasks_failed: u64,
    /// Total errors encountered
    pub errors: u64,
    /// Average event processing time in milliseconds
    pub avg_processing_time_ms: f64,
    /// Timestamp of last activity (Unix millis)
    pub last_activity_ms: u64,
}

/// Enhanced Agent trait with lifecycle and state management
#[async_trait]
pub trait Agent: Send + Sync + Debug {
    /// Get agent metadata
    fn metadata(&self) -> AgentMetadata;

    /// Get current agent status
    fn status(&self) -> AgentStatus;

    /// Get performance metrics
    fn metrics(&self) -> AgentMetrics;

    /// Initialize the agent (called once on startup)
    async fn initialize(&self) -> Result<()> {
        Ok(())
    }

    /// Shutdown the agent (called once on termination)
    async fn shutdown(&self) -> Result<()> {
        Ok(())
    }

    /// Handle an incoming event
    /// Returns a vector of new events that should be published
    /// This allows agents to emit events in response to incoming events
    async fn handle_event(&self, event: &Event) -> Result<Vec<Event>> {
        let _ = event;
        Ok(Vec::new())
    }

    /// Get a snapshot of agent state for debugging
    async fn get_state_snapshot(&self) -> Result<serde_json::Value> {
        Ok(serde_json::json!({
            "id": self.metadata().id,
            "status": self.status().to_string(),
            "metrics": self.metrics(),
        }))
    }
}

/// Agent lifecycle manager
pub struct AgentLifecycleManager {
    agents: std::collections::HashMap<String, Arc<dyn Agent>>,
}

impl AgentLifecycleManager {
    /// Create a new lifecycle manager
    pub fn new() -> Self {
        Self {
            agents: std::collections::HashMap::new(),
        }
    }

    /// Register an agent
    pub fn register(&mut self, agent: Arc<dyn Agent>) -> Result<()> {
        let id = agent.metadata().id.clone();
        self.agents.insert(id, agent);
        Ok(())
    }

    /// Initialize all registered agents
    pub async fn initialize_all(&self) -> Result<()> {
        for agent in self.agents.values() {
            let agent: &dyn Agent = agent.as_ref();
            agent.initialize().await?;
        }
        Ok(())
    }

    /// Shutdown all registered agents
    pub async fn shutdown_all(&self) -> Result<()> {
        for agent in self.agents.values() {
            let agent: &dyn Agent = agent.as_ref();
            agent.shutdown().await?;
        }
        Ok(())
    }

    /// Get an agent by ID
    pub fn get_agent(&self, id: &str) -> Option<Arc<dyn Agent>> {
        self.agents.get(id).cloned()
    }

    /// Get all agents
    pub fn get_all_agents(&self) -> Vec<Arc<dyn Agent>> {
        self.agents.values().cloned().collect()
    }

    /// Get status of all agents
    pub async fn get_all_statuses(&self) -> Vec<(String, AgentStatus)> {
        self.agents
            .iter()
            .map(|(id, agent): (&String, &Arc<dyn Agent>)| (id.clone(), agent.status()))
            .collect()
    }
}

impl Default for AgentLifecycleManager {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[derive(Debug)]
    struct MockAgent {
        id: String,
    }

    #[async_trait]
    impl Agent for MockAgent {
        fn metadata(&self) -> AgentMetadata {
            AgentMetadata {
                id: self.id.clone(),
                name: "Mock Agent".to_string(),
                description: "A mock agent for testing".to_string(),
                version: "1.0.0".to_string(),
                tags: vec!["test".to_string()],
            }
        }

        fn status(&self) -> AgentStatus {
            AgentStatus::Idle
        }

        fn metrics(&self) -> AgentMetrics {
            AgentMetrics::default()
        }
    }

    #[tokio::test]
    async fn test_lifecycle_manager() {
        let mut manager = AgentLifecycleManager::new();
        let agent = Arc::new(MockAgent {
            id: "test_agent".to_string(),
        });

        manager.register(agent.clone()).unwrap();
        assert!(manager.get_agent("test_agent").is_some());

        let agents = manager.get_all_agents();
        assert_eq!(agents.len(), 1);
    }
}
