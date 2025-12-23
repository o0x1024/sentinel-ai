use crate::models::security_testing::*;
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;
use anyhow::{Result, anyhow};

/// Security test session manager
pub struct SecurityTestManager {
    sessions: Arc<RwLock<HashMap<String, TestSession>>>,
}

impl SecurityTestManager {
    pub fn new() -> Self {
        Self {
            sessions: Arc::new(RwLock::new(HashMap::new())),
        }
    }
    
    /// Create a new test session
    pub async fn create_session(
        &self,
        task_kind: SecurityTaskKind,
        primary_target: String,
        summary: String,
    ) -> Result<TestSession> {
        let session = TestSession::new(task_kind, primary_target, summary);
        let session_id = session.id.clone();
        
        let mut sessions = self.sessions.write().await;
        sessions.insert(session_id.clone(), session.clone());
        
        log::info!(
            "Created security test session: id={}, task_kind={:?}, target={}",
            session_id,
            session.task_kind,
            session.primary_target
        );
        
        Ok(session)
    }
    
    /// Create a new test session with task parameters
    pub async fn create_session_with_params(
        &self,
        task_kind: SecurityTaskKind,
        primary_target: String,
        summary: String,
        task_parameters: HashMap<String, serde_json::Value>,
    ) -> Result<TestSession> {
        let mut session = TestSession::new(task_kind, primary_target, summary);
        session.task_parameters = task_parameters;
        let session_id = session.id.clone();
        
        let mut sessions = self.sessions.write().await;
        sessions.insert(session_id.clone(), session.clone());
        
        log::info!(
            "Created security test session with params: id={}, task_kind={:?}, target={}, params_count={}",
            session_id,
            session.task_kind,
            session.primary_target,
            session.task_parameters.len()
        );
        
        Ok(session)
    }
    
    /// Get a session by ID
    pub async fn get_session(&self, session_id: &str) -> Result<TestSession> {
        let sessions = self.sessions.read().await;
        sessions
            .get(session_id)
            .cloned()
            .ok_or_else(|| anyhow!("Session not found: {}", session_id))
    }
    
    /// Update a session
    pub async fn update_session(&self, session: TestSession) -> Result<()> {
        let mut sessions = self.sessions.write().await;
        let session_id = session.id.clone();
        sessions.insert(session_id.clone(), session);
        
        log::debug!("Updated security test session: {}", session_id);
        Ok(())
    }
    
    /// Update session with a closure
    pub async fn update_session_with<F>(&self, session_id: &str, f: F) -> Result<()>
    where
        F: FnOnce(&mut TestSession),
    {
        let mut sessions = self.sessions.write().await;
        let session = sessions
            .get_mut(session_id)
            .ok_or_else(|| anyhow!("Session not found: {}", session_id))?;
        
        f(session);
        
        log::debug!("Updated security test session with closure: {}", session_id);
        Ok(())
    }
    
    /// Add a step to a session
    pub async fn add_step(&self, session_id: &str, step: TestStep) -> Result<()> {
        let mut sessions = self.sessions.write().await;
        let session = sessions
            .get_mut(session_id)
            .ok_or_else(|| anyhow!("Session not found: {}", session_id))?;
        
        session.add_step(step);
        
        log::debug!(
            "Added step to session {}: total steps={}",
            session_id,
            session.steps.len()
        );
        
        Ok(())
    }
    
    /// Update a step in a session
    pub async fn update_step(&self, session_id: &str, step_id: &str, update_fn: impl FnOnce(&mut TestStep)) -> Result<()> {
        let mut sessions = self.sessions.write().await;
        let session = sessions
            .get_mut(session_id)
            .ok_or_else(|| anyhow!("Session not found: {}", session_id))?;
        
        let step = session
            .get_step_by_id_mut(step_id)
            .ok_or_else(|| anyhow!("Step not found: {}", step_id))?;
        
        update_fn(step);
        session.updated_at = chrono::Utc::now();
        
        log::debug!("Updated step {} in session {}", step_id, session_id);
        
        Ok(())
    }
    
    /// Add a finding to a session
    pub async fn add_finding(&self, session_id: &str, finding: Finding) -> Result<()> {
        let mut sessions = self.sessions.write().await;
        let session = sessions
            .get_mut(session_id)
            .ok_or_else(|| anyhow!("Session not found: {}", session_id))?;
        
        let risk_level = finding.risk_level.clone();
        session.add_finding(finding);
        
        log::info!(
            "Added finding to session {}: risk_level={:?}, total findings={}",
            session_id,
            risk_level,
            session.findings.len()
        );
        
        Ok(())
    }
    
    /// Update session stage
    pub async fn update_stage(&self, session_id: &str, stage: TestStage) -> Result<()> {
        let mut sessions = self.sessions.write().await;
        let session = sessions
            .get_mut(session_id)
            .ok_or_else(|| anyhow!("Session not found: {}", session_id))?;
        
        session.update_stage(stage.clone());
        
        log::info!("Updated session {} stage to {:?}", session_id, stage);
        
        Ok(())
    }
    
    /// Update authentication context
    pub async fn update_auth_context(&self, session_id: &str, auth_context: AuthContext) -> Result<()> {
        let mut sessions = self.sessions.write().await;
        let session = sessions
            .get_mut(session_id)
            .ok_or_else(|| anyhow!("Session not found: {}", session_id))?;
        
        session.update_auth_context(auth_context.clone());
        
        log::info!(
            "Updated auth context for session {}: authenticated={}",
            session_id,
            auth_context.is_authenticated()
        );
        
        Ok(())
    }
    
    /// Get session statistics
    pub async fn get_session_stats(&self, session_id: &str) -> Result<SessionStats> {
        let sessions = self.sessions.read().await;
        let session = sessions
            .get(session_id)
            .ok_or_else(|| anyhow!("Session not found: {}", session_id))?;
        
        let total_steps = session.steps.len();
        let completed_steps = session.steps.iter().filter(|s| s.status == StepStatus::Completed).count();
        let failed_steps = session.steps.iter().filter(|s| s.status == StepStatus::Failed).count();
        let running_steps = session.steps.iter().filter(|s| s.status == StepStatus::Running).count();
        
        let total_findings = session.findings.len();
        let critical_findings = session.findings.iter().filter(|f| f.risk_level == RiskImpact::Critical).count();
        let high_findings = session.findings.iter().filter(|f| f.risk_level == RiskImpact::High).count();
        let medium_findings = session.findings.iter().filter(|f| f.risk_level == RiskImpact::Medium).count();
        
        Ok(SessionStats {
            total_steps,
            completed_steps,
            failed_steps,
            running_steps,
            total_findings,
            critical_findings,
            high_findings,
            medium_findings,
        })
    }
    
    /// List all sessions
    pub async fn list_sessions(&self) -> Result<Vec<TestSession>> {
        let sessions = self.sessions.read().await;
        Ok(sessions.values().cloned().collect())
    }
    
    /// Delete a session
    pub async fn delete_session(&self, session_id: &str) -> Result<()> {
        let mut sessions = self.sessions.write().await;
        sessions.remove(session_id)
            .ok_or_else(|| anyhow!("Session not found: {}", session_id))?;
        
        log::info!("Deleted security test session: {}", session_id);
        
        Ok(())
    }
}

impl Default for SecurityTestManager {
    fn default() -> Self {
        Self::new()
    }
}

/// Session statistics
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct SessionStats {
    pub total_steps: usize,
    pub completed_steps: usize,
    pub failed_steps: usize,
    pub running_steps: usize,
    pub total_findings: usize,
    pub critical_findings: usize,
    pub high_findings: usize,
    pub medium_findings: usize,
}

