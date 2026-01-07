use crate::engines::vision_explorer_v2::blackboard::Blackboard;
use crate::engines::vision_explorer_v2::core::{Agent, Event, TaskResult};
use crate::engines::vision_explorer_v2::driver::BrowserActions;
use anyhow::Result;
use async_trait::async_trait;
use std::sync::Arc;
use tokio::sync::{mpsc, Mutex};

/// NavigatorAgent is responsible for moving the browser from point A to point B.
/// It listens for `Navigate` tasks.
pub struct NavigatorAgent {
    id: String,
    driver: Arc<Mutex<dyn BrowserActions>>,
    event_tx: mpsc::Sender<Event>,
    blackboard: Arc<Blackboard>,
}

impl NavigatorAgent {
    pub fn new(
        id: String,
        driver: Arc<Mutex<dyn BrowserActions>>,
        event_tx: mpsc::Sender<Event>,
        blackboard: Arc<Blackboard>,
    ) -> Self {
        Self {
            id,
            driver,
            event_tx,
            blackboard,
        }
    }
}

#[async_trait]
impl Agent for NavigatorAgent {
    fn id(&self) -> String {
        self.id.clone()
    }

    async fn handle_event(&self, event: &Event) -> Result<()> {
        match event {
            Event::TaskAssigned {
                agent_id,
                task_id,
                target_node_id,
                payload,
            } if agent_id == &self.id => {
                let mut action_desc = "Navigation".to_string();
                
                // Execute action and capture result
                let execution_result = async {
                    let driver = self.driver.lock().await;

                    // Check payload for specific action
                    if let Some(payload_val) = payload {
                        if let Ok(action) = serde_json::from_value::<
                            crate::engines::vision_explorer_v2::core::SuggestedAction,
                        >(payload_val.clone())
                        {
                            log::info!(
                                "Executing action: {} on {}",
                                action.action_type,
                                action.selector
                            );
                            action_desc = format!("{} on {}", action.action_type, action.selector);

                            match action.action_type.as_str() {
                                "click" => {
                                    if let (Some(x), Some(y)) = (action.x, action.y) {
                                        log::info!("NavigatorAgent: Clicking coordinate ({}, {})", x, y);
                                        driver.click_coordinate(x, y).await?;
                                    } else {
                                        driver.click(&action.selector).await?;
                                    }
                                }
                                "type" => {
                                    if let Some(val) = &action.value {
                                        if let (Some(x), Some(y)) = (action.x, action.y) {
                                            log::info!("NavigatorAgent: Typing via coordinate click ({}, {})", x, y);
                                            driver.click_coordinate(x, y).await?;
                                            driver.press_key(val, None).await?;
                                        } else {
                                            driver.type_text(&action.selector, val).await?;
                                        }
                                    }
                                }
                                "navigate" => {
                                    driver.goto(&action.selector).await?;
                                }
                                "fill_form" => {
                                    if let Some(val) = &action.value {
                                        if let Ok(data) = serde_json::from_str::<serde_json::Value>(val) {
                                            if let Some(map) = data.as_object() {
                                                let creds = self.blackboard.get_credentials().await;
                                                for (sel, field_val) in map {
                                                    if let Some(text) = field_val.as_str() {
                                                        let resolved_text = if text == "[USERNAME]" {
                                                            creds.as_ref().map(|c| c.username.as_str()).unwrap_or("")
                                                        } else if text == "[PASSWORD]" {
                                                            creds.as_ref().map(|c| c.password.as_str()).unwrap_or("")
                                                        } else {
                                                            text
                                                        };
                                                        if !resolved_text.is_empty() {
                                                            driver.type_text(sel, resolved_text).await?;
                                                        }
                                                    }
                                                }
                                            }
                                        }
                                    }
                                }
                                _ => {
                                    log::warn!("Unknown action type: {}", action.action_type);
                                }
                            }
                        }
                    } else if target_node_id.starts_with("http") {
                        action_desc = format!("Navigate to {}", target_node_id);
                        driver.goto(target_node_id).await?;
                    }

                    // Capture Context AFTER action
                    let mut context = driver.capture_context().await?;
                    
                    // Fallback: If URL is empty but we just navigated, use the target URL
                    if context.url.is_empty() {
                         if let Some(payload_val) = payload {
                            if let Ok(action) = serde_json::from_value::<
                                crate::engines::vision_explorer_v2::core::SuggestedAction,
                            >(payload_val.clone()) {
                                if action.action_type == "navigate" {
                                    log::warn!("NavigatorAgent: Captured URL is empty, falling back to action target: {}", action.selector);
                                    context.url = action.selector.clone();
                                }
                            }
                        } else if target_node_id.starts_with("http") {
                             log::warn!("NavigatorAgent: Captured URL is empty, falling back to target_node_id: {}", target_node_id);
                             context.url = target_node_id.clone();
                        }
                    }
                    
                    Ok::<_, anyhow::Error>(context)
                }.await;

                // Always send TaskCompleted, even on error
                let result = match execution_result {
                    Ok(context) => {
                        log::info!("NavigatorAgent: Task completed successfully: {}", action_desc);
                        TaskResult {
                            success: true,
                            message: format!("Completed: {}", action_desc),
                            new_nodes: vec![],
                            data: Some(serde_json::to_value(context)?),
                        }
                    }
                    Err(e) => {
                        log::error!("NavigatorAgent: Task failed: {} - Error: {}", action_desc, e);
                        TaskResult {
                            success: false,
                            message: format!("Failed: {} - {}", action_desc, e),
                            new_nodes: vec![],
                            data: None,
                        }
                    }
                };

                // Send completion event
                self.event_tx
                    .send(Event::TaskCompleted {
                        agent_id: self.id.clone(),
                        task_id: task_id.clone(),
                        result,
                    })
                    .await?;

                Ok(())
            }
            _ => Ok(()),
        }
    }
}
