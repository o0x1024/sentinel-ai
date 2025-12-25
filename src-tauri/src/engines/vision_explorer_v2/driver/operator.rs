use crate::engines::vision_explorer_v2::core::{Agent, Event, TaskResult};
use crate::engines::vision_explorer_v2::driver::BrowserActions;
use anyhow::Result;
use async_trait::async_trait;
use std::sync::Arc;
use tokio::sync::{mpsc, Mutex};

/// OperatorAgent is responsible for complex interactions (The Hand).
/// It handles multi-step forms, file uploads, and auth flows.
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
}

#[async_trait]
impl Agent for OperatorAgent {
    fn id(&self) -> String {
        self.id.clone()
    }

    async fn handle_event(&self, event: &Event) -> Result<()> {
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
                        // Expecting something like { "operation": "fill_form", "data": {...} }
                        if let Some(op) = val.get("operation").and_then(|v| v.as_str()) {
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
                                            // TODO: Implement proper file upload when available in BrowserDriver
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
                self.event_tx
                    .send(Event::TaskCompleted {
                        agent_id: self.id.clone(),
                        task_id: task_id.clone(),
                        result: TaskResult {
                            success,
                            message,
                            new_nodes: vec![],
                            data: None,
                        },
                    })
                    .await?;

                Ok(())
            }
            _ => Ok(()),
        }
    }
}
