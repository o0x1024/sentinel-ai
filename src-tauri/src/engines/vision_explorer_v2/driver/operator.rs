use crate::engines::vision_explorer_v2::agent_framework::{Agent, AgentMetadata, AgentMetrics, AgentStatus};
use crate::engines::vision_explorer_v2::core::{Event, SuggestedAction, TaskResult};
use crate::engines::vision_explorer_v2::driver::BrowserActions;
use anyhow::Result;
use async_trait::async_trait;
use std::sync::Arc;
use tokio::sync::{mpsc, Mutex};

/// OperatorAgent is responsible for complex interactions (The Hand).
/// It handles multi-step forms, clicks, typing, and other UI interactions.
pub struct OperatorAgent {
    id: String,
    driver: Arc<Mutex<dyn BrowserActions>>,
    event_tx: mpsc::Sender<Event>,
}

impl OperatorAgent {
    pub fn new(
        id: String,
        driver: Arc<Mutex<dyn BrowserActions>>,
        event_tx: mpsc::Sender<Event>,
    ) -> Self {
        Self {
            id,
            driver,
            event_tx,
        }
    }

    /// Perform a single suggested action
    async fn perform_suggested_action(&self, driver: &dyn BrowserActions, action: &SuggestedAction) -> Result<String> {
        log::info!("OperatorAgent: Performing action: {} ({:?})", action.description, action.action_type);

        match action.action_type.as_str() {
            "click" => {
                if let (Some(x), Some(y)) = (action.x, action.y) {
                    log::debug!("OperatorAgent: Clicking coordinate ({}, {})", x, y);
                    driver.click_coordinate(x, y).await?;
                    Ok(format!("Clicked at ({}, {})", x, y))
                } else if !action.selector.is_empty() {
                    log::debug!("OperatorAgent: Clicking selector '{}'", action.selector);
                    driver.click(&action.selector).await?;
                    Ok(format!("Clicked selector '{}'", action.selector))
                } else {
                    Err(anyhow::anyhow!("Click action missing both coordinates and selector"))
                }
            }
            "type" => {
                let text = action.value.as_deref().unwrap_or("");
                if !action.selector.is_empty() {
                    log::debug!("OperatorAgent: Typing '{}' into '{}'", text, action.selector);
                    driver.type_text(&action.selector, text).await?;
                    Ok(format!("Typed into '{}'", action.selector))
                } else if let (Some(x), Some(y)) = (action.x, action.y) {
                    log::debug!("OperatorAgent: Typing via coordinate click ({}, {})", x, y);
                    driver.click_coordinate(x, y).await?;
                    // For pure vision typing, we might need a more advanced strategy
                    // but for now we just click and type if the driver supports it
                    // or use a generic press_key if we clicked the focus
                    driver.press_key(text, None).await?;
                    Ok(format!("Clicked ({}, {}) and typed text", x, y))
                } else {
                    Err(anyhow::anyhow!("Type action missing both coordinates and selector"))
                }
            }
            "scroll" => {
                // Simplified scroll
                driver.press_key("PageDown", None).await?;
                Ok("Scrolled down".to_string())
            }
            "hover" => {
                if let (Some(x), Some(y)) = (action.x, action.y) {
                    // Hover coordinate not yet in BrowserActions but we can simulate
                    Ok(format!("Hover at ({}, {}) [simulated]", x, y))
                } else if !action.selector.is_empty() {
                    driver.hover(&action.selector).await?;
                    Ok(format!("Hovered over '{}'", action.selector))
                } else {
                    Ok("Hover missing target".to_string())
                }
            }
            _ => Err(anyhow::anyhow!("Unsupported action type: {}", action.action_type)),
        }
    }
}

impl std::fmt::Debug for OperatorAgent {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("OperatorAgent")
            .field("id", &self.id)
            .field("driver", &"<BrowserActions>")
            .field("event_tx", &self.event_tx)
            .finish()
    }
}

#[async_trait]
impl Agent for OperatorAgent {
    fn metadata(&self) -> AgentMetadata {
        AgentMetadata {
            id: self.id.clone(),
            name: "Operator Agent".to_string(),
            description: "Handles complex browser interactions and operations".to_string(),
            version: "1.0.0".to_string(),
            tags: vec!["operator".to_string(), "interaction".to_string(), "forms".to_string()],
        }
    }

    fn status(&self) -> AgentStatus {
        AgentStatus::Idle
    }

    fn metrics(&self) -> AgentMetrics {
        AgentMetrics::default()
    }

    async fn handle_event(&self, event: &Event) -> Result<Vec<Event>> {
        match event {
            Event::TaskAssigned {
                agent_id,
                task_id,
                payload,
                ..
            } if agent_id == &self.id => {
                // Execute operation and capture result
                let execution_result = async {
                    let driver = self.driver.lock().await;
                    let mut message = "No op".to_string();

                    if let Some(val) = payload {
                        // 1. Check if it's a SuggestedAction
                        if let Ok(action) = serde_json::from_value::<SuggestedAction>(val.clone()) {
                            message = self.perform_suggested_action(&*driver, &action).await?;
                        }
                        // 2. Fallback to legacy operation format
                        else if let Some(op) = val.get("operation").and_then(|v| v.as_str()) {
                            match op {
                                "fill_form" => {
                                    if let Some(data) = val.get("data").and_then(|v| v.as_object()) {
                                        let mut filled_count = 0;
                                        let mut errors = Vec::new();
                                        for (selector, value) in data {
                                            if let Some(text) = value.as_str() {
                                                if let Err(e) = driver.type_text(selector, text).await {
                                                    log::warn!(
                                                        "Failed to type into {}: {}",
                                                        selector,
                                                        e
                                                    );
                                                    errors.push(format!("{}: {}", selector, e));
                                                } else {
                                                    filled_count += 1;
                                                }
                                            }
                                        }
                                        if errors.is_empty() {
                                            message = format!("Filled {} form fields", filled_count);
                                        } else {
                                            message = format!("Filled {} fields, {} errors: {}", 
                                                filled_count, errors.len(), errors.join(", "));
                                        }
                                    } else {
                                        message = "Missing data for fill_form".to_string();
                                    }
                                }
                                "upload_file" => {
                                    if let Some(selector) = val.get("selector").and_then(|v| v.as_str())
                                    {
                                        if let Some(path) = val.get("path").and_then(|v| v.as_str()) {
                                            log::info!("Pretending to upload {} to {}", path, selector);
                                            message = format!("File upload simulated: {}", path);
                                        } else {
                                            message = "Missing path for upload_file".to_string();
                                        }
                                    } else {
                                        message = "Missing selector for upload_file".to_string();
                                    }
                                }
                                _ => {
                                    log::warn!("Unknown operator operation: {}", op);
                                    message = format!("Unknown operation: {}", op);
                                }
                            }
                        } else {
                            message = "Invalid payload for OperatorAgent".to_string();
                        }
                    }
                    Ok::<_, anyhow::Error>(message)
                }.await;

                // Handle execution result
                let (message, success) = match execution_result {
                    Ok(msg) => {
                        log::info!("OperatorAgent: Task completed successfully: {}", msg);
                        (msg, true)
                    }
                    Err(e) => {
                        log::error!("OperatorAgent: Task failed: {}", e);
                        (format!("Operation failed: {}", e), false)
                    }
                };

                // Always send TaskCompleted
                let task_completed = Event::TaskCompleted {
                    agent_id: self.id.clone(),
                    task_id: task_id.clone(),
                    result: TaskResult {
                        success,
                        message,
                        new_nodes: vec![],
                        data: None,
                    },
                };

                if let Err(e) = self.event_tx.send(task_completed.clone()).await {
                    log::error!("OperatorAgent: Failed to send TaskCompleted: {}", e);
                }

                Ok(vec![task_completed])
            }
            _ => Ok(vec![]),
        }
    }
}
