//! Terminal session manager

use super::session::{TerminalSession, TerminalSessionConfig, SessionState};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::{mpsc, RwLock};
use tracing::{info, warn};

/// Terminal session manager
pub struct TerminalSessionManager {
    sessions: Arc<RwLock<HashMap<String, Arc<RwLock<TerminalSession>>>>>,
    cleanup_interval: std::time::Duration,
    max_idle_duration: std::time::Duration,
}

impl TerminalSessionManager {
    /// Create a new session manager
    pub fn new() -> Self {
        let manager = Self {
            sessions: Arc::new(RwLock::new(HashMap::new())),
            cleanup_interval: std::time::Duration::from_secs(60),
            max_idle_duration: std::time::Duration::from_secs(1800), // 30 minutes
        };

        // Start cleanup task
        manager.start_cleanup_task();

        manager
    }

    /// Create a new terminal session
    pub async fn create_session(
        &self,
        config: TerminalSessionConfig,
    ) -> Result<(String, mpsc::UnboundedReceiver<Vec<u8>>), String> {
        let session_id = uuid::Uuid::new_v4().to_string();
        let mut session = TerminalSession::new(session_id.clone(), config).await?;

        let (output_tx, output_rx) = mpsc::unbounded_channel();

        session.start(output_tx).await?;

        let mut sessions = self.sessions.write().await;
        sessions.insert(session_id.clone(), Arc::new(RwLock::new(session)));

        info!("Created terminal session: {}", session_id);
        Ok((session_id, output_rx))
    }

    /// Get a session
    pub async fn get_session(&self, session_id: &str) -> Option<Arc<RwLock<TerminalSession>>> {
        let sessions = self.sessions.read().await;
        sessions.get(session_id).cloned()
    }

    /// Write data to a session
    pub async fn write_to_session(&self, session_id: &str, data: Vec<u8>) -> Result<(), String> {
        let session = self
            .get_session(session_id)
            .await
            .ok_or_else(|| "Session not found".to_string())?;

        let session = session.read().await;
        session.write(data).await
    }

    /// Touch session to keep it active
    pub async fn touch_session(&self, session_id: &str) -> Result<(), String> {
        let session = self
            .get_session(session_id)
            .await
            .ok_or_else(|| "Session not found".to_string())?;

        let session = session.read().await;
        session.touch().await;
        Ok(())
    }

    /// Stop a session
    pub async fn stop_session(&self, session_id: &str) -> Result<(), String> {
        let session = {
            let mut sessions = self.sessions.write().await;
            sessions.remove(session_id)
        };

        if let Some(session) = session {
            let mut session = session.write().await;
            session.stop().await?;
            info!("Stopped terminal session: {}", session_id);
            Ok(())
        } else {
            Err("Session not found".to_string())
        }
    }

    /// List all sessions
    pub async fn list_sessions(&self) -> Vec<SessionInfo> {
        let sessions = self.sessions.read().await;
        let mut infos = Vec::new();

        for (id, session) in sessions.iter() {
            let session = session.read().await;
            infos.push(SessionInfo {
                id: id.clone(),
                state: session.state().await,
                last_activity: session.last_activity().await.elapsed().as_secs(),
                use_docker: session.config.use_docker,
            });
        }

        infos
    }

    /// Start cleanup task
    fn start_cleanup_task(&self) {
        let sessions = self.sessions.clone();
        let cleanup_interval = self.cleanup_interval;
        let max_idle_duration = self.max_idle_duration;

        tokio::spawn(async move {
            let mut interval = tokio::time::interval(cleanup_interval);
            loop {
                interval.tick().await;

                let mut sessions_to_remove = Vec::new();
                {
                    let sessions_guard = sessions.read().await;
                    for (id, session) in sessions_guard.iter() {
                        let session_guard = session.read().await;
                        let idle_duration = session_guard.last_activity().await.elapsed();

                        if idle_duration > max_idle_duration {
                            warn!(
                                "Session {} idle for {:?}, marking for cleanup",
                                id, idle_duration
                            );
                            sessions_to_remove.push(id.clone());
                        }
                    }
                }

                // Remove idle sessions
                if !sessions_to_remove.is_empty() {
                    let mut sessions_guard = sessions.write().await;
                    for id in sessions_to_remove {
                        if let Some(session) = sessions_guard.remove(&id) {
                            let mut session_guard = session.write().await;
                            let _ = session_guard.stop().await;
                            info!("Cleaned up idle session: {}", id);
                        }
                    }
                }
            }
        });
    }

    /// Get session count
    pub async fn session_count(&self) -> usize {
        self.sessions.read().await.len()
    }

    /// Clean up unused containers (manually invoked)
    pub async fn cleanup_containers(&self) -> Result<Vec<String>, String> {
        use tokio::process::Command;
        
        info!("Cleaning up unused sentinel-sandbox containers");
        
        // Get all sentinel-sandbox containers
        let output = Command::new("docker")
            .args(&[
                "ps",
                "-a",
                "--filter",
                "name=^sentinel-sandbox",
                "--format",
                "{{.ID}}",
            ])
            .output()
            .await
            .map_err(|e| format!("Failed to list containers: {}", e))?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            return Err(format!("Docker ps failed: {}", stderr));
        }

        let container_ids: Vec<String> = String::from_utf8_lossy(&output.stdout)
            .lines()
            .map(|s| s.trim().to_string())
            .filter(|s| !s.is_empty())
            .collect();

        let mut removed = Vec::new();

        // Check which containers are in use by active sessions
        let sessions = self.sessions.read().await;
        let active_containers: Vec<String> = {
            let mut containers = Vec::new();
            for session in sessions.values() {
                let session_guard = session.read().await;
                if let Some(cid) = session_guard.container_id() {
                    containers.push(cid);
                }
            }
            containers
        };

        // Remove containers not in use
        for container_id in container_ids {
            if !active_containers.contains(&container_id) {
                info!("Removing unused container: {}", container_id);
                let result = Command::new("docker")
                    .args(&["rm", "-f", &container_id])
                    .output()
                    .await;

                if result.is_ok() {
                    removed.push(container_id);
                }
            }
        }

        info!("Cleaned up {} unused containers", removed.len());
        Ok(removed)
    }

    /// Get container info for all sessions
    pub async fn get_container_info(&self) -> Vec<ContainerInfo> {
        let sessions = self.sessions.read().await;
        let mut infos = Vec::new();

        for (id, session) in sessions.iter() {
            let session_guard = session.read().await;
            if let Some(container_id) = session_guard.container_id() {
                let is_healthy = session_guard.is_container_healthy().await;
                infos.push(ContainerInfo {
                    session_id: id.clone(),
                    container_id,
                    is_healthy,
                });
            }
        }

        infos
    }
}

/// Container information
#[derive(Debug, Clone, serde::Serialize)]
pub struct ContainerInfo {
    pub session_id: String,
    pub container_id: String,
    pub is_healthy: bool,
}

impl Default for TerminalSessionManager {
    fn default() -> Self {
        Self::new()
    }
}

/// Session information
#[derive(Debug, Clone, serde::Serialize)]
pub struct SessionInfo {
    pub id: String,
    pub state: SessionState,
    pub last_activity: u64,
    pub use_docker: bool,
}
