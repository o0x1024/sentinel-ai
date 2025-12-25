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
                let driver = self.driver.lock().await;

                // Track what we did for the message
                let mut action_desc = "Navigation".to_string();

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
                                driver.click(&action.selector).await?;
                            }
                            "type" => {
                                if let Some(val) = &action.value {
                                    driver.type_text(&action.selector, val).await?;
                                }
                            }
                            "navigate" => {
                                driver.goto(&action.selector).await?; // Treat selector as URL for navigate action
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
                let context = driver.capture_context().await?;
                drop(driver); // Release lock

                //Emit TaskCompleted
                self.event_tx
                    .send(Event::TaskCompleted {
                        agent_id: self.id.clone(),
                        task_id: task_id.clone(),
                        result: TaskResult {
                            success: true,
                            message: format!("Completed: {}", action_desc),
                            new_nodes: vec![],
                            data: Some(serde_json::to_value(context)?),
                        },
                    })
                    .await?;

                Ok(())
            }
            _ => Ok(()),
        }
    }
}
