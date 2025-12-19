//! V2 Message Emitter - Frontend communication for Vision Explorer V2
//!
//! This module provides event emission for the V2 Engine to communicate with
//! the frontend. It uses the same event format as V1 for compatibility.

use crate::engines::vision_explorer_v2::core::{Event, PageContext, SuggestedAction, TaskResult};
use crate::engines::vision_explorer_v2::persistence::ExplorationStats;
use serde::Serialize;
use std::sync::Arc;
use tauri::{AppHandle, Emitter};
use tracing::debug;

/// V2 Message Emitter - Sends events to the frontend
pub struct V2MessageEmitter {
    app_handle: Arc<AppHandle>,
    execution_id: String,
    session_id: String,
}

/// Vision step for frontend display (compatible with V1 format)
#[derive(Debug, Clone, Serialize)]
pub struct VisionStepV2 {
    pub iteration: u32,
    pub phase: String,
    pub status: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub url: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub title: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub screenshot: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub analysis: Option<VisionAnalysisV2>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub action: Option<VisionActionV2>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
}

/// VLM analysis result
#[derive(Debug, Clone, Serialize)]
pub struct VisionAnalysisV2 {
    pub page_analysis: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub estimated_apis: Option<Vec<String>>,
    pub exploration_progress: f32,
}

/// Action execution info
#[derive(Debug, Clone, Serialize)]
pub struct VisionActionV2 {
    pub action_type: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub element_index: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub value: Option<String>,
    pub reason: String,
    pub success: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub duration_ms: Option<u64>,
}

/// Exploration completion stats
#[derive(Debug, Clone, Serialize)]
pub struct VisionCompletionStats {
    pub total_iterations: u32,
    pub pages_visited: u32,
    pub apis_discovered: u32,
    pub elements_interacted: u32,
    pub total_duration_ms: u64,
    pub status: String,
}

impl V2MessageEmitter {
    pub fn new(app_handle: Arc<AppHandle>, execution_id: String, session_id: String) -> Self {
        Self {
            app_handle,
            execution_id,
            session_id,
        }
    }

    /// Emit exploration start event
    pub fn emit_start(&self, target_url: &str) {
        let payload = serde_json::json!({
            "execution_id": self.execution_id,
            "session_id": self.session_id,
            "target_url": target_url,
            "engine": "v2"
        });

        if let Err(e) = self.app_handle.emit("vision:start", &payload) {
            debug!("Failed to emit vision:start: {}", e);
        }
    }

    /// Emit screenshot captured event
    pub fn emit_screenshot(
        &self,
        iteration: u32,
        url: &str,
        title: &str,
        screenshot_base64: Option<&str>,
    ) {
        let step = VisionStepV2 {
            iteration,
            phase: "screenshot".to_string(),
            status: "completed".to_string(),
            url: Some(url.to_string()),
            title: Some(title.to_string()),
            screenshot: screenshot_base64.map(|s| s.to_string()),
            analysis: None,
            action: None,
            error: None,
        };

        self.emit_step(&step);

        // Also emit dedicated screenshot event for VisionExplorerProgress
        let payload = serde_json::json!({
            "execution_id": self.execution_id,
            "iteration": iteration,
            "url": url,
            "title": title,
            "path": "", // V2 uses base64, not file path
            "has_screenshot": screenshot_base64.is_some()
        });

        if let Err(e) = self.app_handle.emit("vision:screenshot", &payload) {
            debug!("Failed to emit vision:screenshot: {}", e);
        }
    }

    /// Emit analysis result event
    pub fn emit_analysis(
        &self,
        iteration: u32,
        summary: &str,
        suggested_actions: &[SuggestedAction],
        progress: f32,
    ) {
        let estimated_apis: Vec<String> = suggested_actions
            .iter()
            .filter(|a| a.action_type == "api" || a.description.contains("API"))
            .map(|a| a.selector.clone())
            .collect();

        let analysis = VisionAnalysisV2 {
            page_analysis: summary.to_string(),
            estimated_apis: if estimated_apis.is_empty() {
                None
            } else {
                Some(estimated_apis)
            },
            exploration_progress: progress,
        };

        let step = VisionStepV2 {
            iteration,
            phase: "analyze".to_string(),
            status: "completed".to_string(),
            url: None,
            title: None,
            screenshot: None,
            analysis: Some(analysis.clone()),
            action: None,
            error: None,
        };

        self.emit_step(&step);

        // Dedicated analysis event
        let payload = serde_json::json!({
            "execution_id": self.execution_id,
            "iteration": iteration,
            "analysis": {
                "page_analysis": summary,
                "estimated_apis": analysis.estimated_apis,
                "exploration_progress": progress
            }
        });

        if let Err(e) = self.app_handle.emit("vision:analysis", &payload) {
            debug!("Failed to emit vision:analysis: {}", e);
        }
    }

    /// Emit action execution event
    pub fn emit_action(
        &self,
        iteration: u32,
        action: &SuggestedAction,
        success: bool,
        duration_ms: Option<u64>,
    ) {
        let action_v2 = VisionActionV2 {
            action_type: action.action_type.clone(),
            element_index: None, // V2 uses selector, not index
            value: action.value.clone(),
            reason: action.description.clone(),
            success,
            duration_ms,
        };

        let step = VisionStepV2 {
            iteration,
            phase: "action".to_string(),
            status: if success {
                "completed".to_string()
            } else {
                "failed".to_string()
            },
            url: None,
            title: None,
            screenshot: None,
            analysis: None,
            action: Some(action_v2.clone()),
            error: None,
        };

        self.emit_step(&step);

        // Dedicated action event
        let payload = serde_json::json!({
            "execution_id": self.execution_id,
            "iteration": iteration,
            "action": {
                "action_type": action.action_type,
                "selector": action.selector,
                "value": action.value,
                "reason": action.description,
                "success": success,
                "duration_ms": duration_ms
            }
        });

        if let Err(e) = self.app_handle.emit("vision:action", &payload) {
            debug!("Failed to emit vision:action: {}", e);
        }
    }

    /// Emit error event
    pub fn emit_error(&self, iteration: u32, error: &str) {
        let step = VisionStepV2 {
            iteration,
            phase: "error".to_string(),
            status: "failed".to_string(),
            url: None,
            title: None,
            screenshot: None,
            analysis: None,
            action: None,
            error: Some(error.to_string()),
        };

        self.emit_step(&step);

        let payload = serde_json::json!({
            "execution_id": self.execution_id,
            "iteration": iteration,
            "error": error
        });

        if let Err(e) = self.app_handle.emit("vision:error", &payload) {
            debug!("Failed to emit vision:error: {}", e);
        }
    }

    /// Emit exploration complete event
    pub fn emit_complete(&self, stats: &ExplorationStats, status: &str, duration_ms: u64) {
        let completion = VisionCompletionStats {
            total_iterations: stats.actions_performed,
            pages_visited: stats.pages_visited,
            apis_discovered: stats.apis_discovered,
            elements_interacted: stats.actions_performed,
            total_duration_ms: duration_ms,
            status: status.to_string(),
        };

        let payload = serde_json::json!({
            "execution_id": self.execution_id,
            "session_id": self.session_id,
            "stats": {
                "total_iterations": completion.total_iterations,
                "pages_visited": completion.pages_visited,
                "apis_discovered": completion.apis_discovered,
                "elements_interacted": completion.elements_interacted,
                "total_duration_ms": completion.total_duration_ms,
                "status": completion.status
            }
        });

        if let Err(e) = self.app_handle.emit("vision:complete", &payload) {
            debug!("Failed to emit vision:complete: {}", e);
        }
    }

    /// Emit login takeover request
    pub fn emit_takeover_request(&self, message: &str) {
        let payload = serde_json::json!({
            "execution_id": self.execution_id,
            "message": message,
            "fields": [
                {"id": "username", "label": "Username", "field_type": "text", "required": true},
                {"id": "password", "label": "Password", "field_type": "password", "required": true}
            ]
        });

        if let Err(e) = self.app_handle.emit("vision:takeover_request", &payload) {
            debug!("Failed to emit vision:takeover_request: {}", e);
        }
    }

    /// Emit credentials received confirmation
    pub fn emit_credentials_received(&self, username: &str) {
        let payload = serde_json::json!({
            "execution_id": self.execution_id,
            "username": username,
            "message": format!("Credentials received for user {}", username)
        });

        if let Err(e) = self
            .app_handle
            .emit("vision:credentials_received", &payload)
        {
            debug!("Failed to emit vision:credentials_received: {}", e);
        }
    }

    /// Emit a V2-specific event (for graph state updates, etc.)
    pub fn emit_v2_event(&self, event_type: &str, data: &serde_json::Value) {
        let payload = serde_json::json!({
            "execution_id": self.execution_id,
            "session_id": self.session_id,
            "type": event_type,
            "data": data
        });

        if let Err(e) = self.app_handle.emit("vision:v2", &payload) {
            debug!("Failed to emit vision:v2 {}: {}", event_type, e);
        }
    }

    /// Emit graph update (nodes/edges discovered)
    pub fn emit_graph_update(
        &self,
        nodes_count: usize,
        edges_count: usize,
        current_node: Option<&str>,
    ) {
        let payload = serde_json::json!({
            "execution_id": self.execution_id,
            "session_id": self.session_id,
            "nodes_count": nodes_count,
            "edges_count": edges_count,
            "current_node": current_node
        });

        if let Err(e) = self.app_handle.emit("vision:graph_update", &payload) {
            debug!("Failed to emit vision:graph_update: {}", e);
        }
    }

    /// Emit log message
    pub fn emit_log(&self, level: &str, message: &str) {
        let payload = serde_json::json!({
            "execution_id": self.execution_id,
            "level": level,
            "message": message,
            "timestamp": chrono::Utc::now().to_rfc3339()
        });

        // Use the existing vision:multi_agent channel for logs to reuse frontend listener
        if let Err(e) = self.app_handle.emit("vision:log", &payload) {
            debug!("Failed to emit vision:log: {}", e);
        }
    }

    /// Internal: emit a vision step
    fn emit_step(&self, step: &VisionStepV2) {
        let payload = serde_json::json!({
            "execution_id": self.execution_id,
            "step": step
        });

        // Emit as vision_step for VisionExplorerPanel timeline
        if let Err(e) = self.app_handle.emit("vision:step", &payload) {
            debug!("Failed to emit vision:step: {}", e);
        }
    }
}
