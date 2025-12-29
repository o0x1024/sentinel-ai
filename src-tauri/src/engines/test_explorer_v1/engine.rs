//! Execution engine for Test Explorer V1

use anyhow::{anyhow, Result};
use std::sync::Arc;
use std::time::{Duration, Instant, SystemTime};
use tokio::sync::RwLock;
use tracing::{debug, error, info, warn};

use super::driver::BrowserDriver;
use super::planner::TaskPlanner;
use super::types::{
    Action, ActionType, ExecutionResult, ExecutionStep, PageState, StreamEvent,
    TestExplorerV1Config,
};

/// Main execution engine
pub struct TestExplorerV1Engine {
    config: TestExplorerV1Config,
    driver: Arc<BrowserDriver>,
    planner: Option<TaskPlanner>,
    execution_history: Vec<ExecutionStep>,
    step_counter: usize,
}

impl TestExplorerV1Engine {
    /// Create a new engine
    pub async fn new(config: TestExplorerV1Config, planner: Option<TaskPlanner>) -> Result<Self> {
        info!("Initializing TestExplorerV1Engine");

        // Initialize browser driver
        let driver = BrowserDriver::new(config.clone()).await?;

        // Start network capture if enabled
        if config.capture_network {
            driver.start_network_capture().await?;
        }

        Ok(Self {
            config,
            driver: Arc::new(driver),
            planner,
            execution_history: Vec::new(),
            step_counter: 0,
        })
    }

    /// Execute in direct mode (LLM + tools)
    /// This mode relies on the LLM to call tools directly through the tool system
    pub async fn execute_direct(&mut self, user_prompt: &str) -> Result<ExecutionResult> {
        info!("Executing in direct mode: {}", user_prompt);

        let start_time = Instant::now();

        // Navigate to target URL first
        let initial_state = self.driver.navigate(&self.config.target_url).await?;

        // In direct mode, we don't execute actions here
        // Instead, the LLM will call tools directly through the tool system
        // This method just returns the initial state
        
        let final_state = self.driver.get_page_state().await?;
        let captured_apis = self.driver.get_captured_requests().await;

        Ok(ExecutionResult {
            success: true,
            final_state,
            captured_apis,
            extracted_data: None,
            steps_taken: self.execution_history.clone(),
            total_duration_ms: start_time.elapsed().as_millis() as u64,
            error: None,
        })
    }

    /// Execute with planning mode (task decomposition + execution)
    pub async fn execute_with_planning(&mut self, user_goal: &str) -> Result<ExecutionResult> {
        info!("Executing with planning mode: {}", user_goal);

        let start_time = Instant::now();

        // Navigate to target URL
        let mut current_state = self.driver.navigate(&self.config.target_url).await?;

        // Check planner exists
        if self.planner.is_none() {
            return Err(anyhow!("Planner not initialized"));
        }

        // Decompose task
        let mut actions = {
            let planner = self.planner.as_ref().unwrap();
            planner.decompose_task(user_goal, &current_state).await?
        };

        // Execute actions
        while !actions.is_empty() && self.step_counter < self.config.max_steps as usize {
            let action = actions.remove(0);

            match self.execute_action(&action).await {
                Ok(state) => {
                    current_state = state;
                }
                Err(e) => {
                    error!("Action failed: {}", e);

                    // Try to replan
                    let new_actions = {
                        let planner = self.planner.as_ref().unwrap();
                        planner.replan(&action, &e.to_string(), &current_state).await
                    };
                    
                    match new_actions {
                        Ok(new_actions) => {
                            info!("Replanned with {} new actions", new_actions.len());
                            actions = new_actions;
                        }
                        Err(replan_err) => {
                            error!("Replan failed: {}", replan_err);
                            break;
                        }
                    }
                }
            }

            // Check if we should finish
            if action.action_type == ActionType::Finish {
                break;
            }
        }

        let final_state = self.driver.get_page_state().await?;
        let captured_apis = self.driver.get_captured_requests().await;

        Ok(ExecutionResult {
            success: true,
            final_state,
            captured_apis,
            extracted_data: None,
            steps_taken: self.execution_history.clone(),
            total_duration_ms: start_time.elapsed().as_millis() as u64,
            error: None,
        })
    }

    /// Execute with streaming events
    pub async fn execute_streaming<F>(
        &mut self,
        user_goal: &str,
        use_planner: bool,
        mut on_event: F,
    ) -> Result<ExecutionResult>
    where
        F: FnMut(StreamEvent),
    {
        info!("Executing with streaming: {}", user_goal);

        let start_time = Instant::now();

        // Navigate to target URL
        on_event(StreamEvent::Log {
            level: "info".to_string(),
            message: format!("Navigating to {}", self.config.target_url),
        });

        let mut current_state = self.driver.navigate(&self.config.target_url).await?;

        on_event(StreamEvent::PageStateUpdated {
            state: current_state.clone(),
        });

        if use_planner {
            if self.planner.is_none() {
                return Err(anyhow!("Planner not initialized"));
            }

            // Decompose task
            on_event(StreamEvent::Log {
                level: "info".to_string(),
                message: "Decomposing task into actions...".to_string(),
            });

            let mut actions = {
                let planner = self.planner.as_ref().unwrap();
                planner.decompose_task(user_goal, &current_state).await?
            };

            // Execute actions
            while !actions.is_empty() && self.step_counter < self.config.max_steps as usize {
                let action = actions.remove(0);

                on_event(StreamEvent::StepStarted {
                    step_number: self.step_counter + 1,
                    action: action.clone(),
                });

                match self.execute_action(&action).await {
                    Ok(state) => {
                        current_state = state.clone();

                        on_event(StreamEvent::StepCompleted {
                            step_number: self.step_counter,
                            success: true,
                            error: None,
                        });

                        on_event(StreamEvent::PageStateUpdated { state });

                        // Emit captured APIs
                        let apis = self.driver.get_captured_requests().await;
                        for api in apis.iter().skip(current_state.captured_apis.len()) {
                            on_event(StreamEvent::ApiCaptured {
                                request: api.clone(),
                            });
                        }
                    }
                    Err(e) => {
                        on_event(StreamEvent::StepCompleted {
                            step_number: self.step_counter,
                            success: false,
                            error: Some(e.to_string()),
                        });

                        // Try to replan
                        let new_actions = {
                            let planner = self.planner.as_ref().unwrap();
                            planner.replan(&action, &e.to_string(), &current_state).await
                        };
                        
                        match new_actions {
                            Ok(new_actions) => {
                                on_event(StreamEvent::Log {
                                    level: "info".to_string(),
                                    message: format!("Replanned with {} new actions", new_actions.len()),
                                });
                                actions = new_actions;
                            }
                            Err(_) => break,
                        }
                    }
                }

                if action.action_type == ActionType::Finish {
                    break;
                }
            }
        }

        let final_state = self.driver.get_page_state().await?;
        let captured_apis = self.driver.get_captured_requests().await;

        let result = ExecutionResult {
            success: true,
            final_state,
            captured_apis,
            extracted_data: None,
            steps_taken: self.execution_history.clone(),
            total_duration_ms: start_time.elapsed().as_millis() as u64,
            error: None,
        };

        on_event(StreamEvent::Finished {
            result: result.clone(),
        });

        Ok(result)
    }

    /// Execute a single action
    async fn execute_action(&mut self, action: &Action) -> Result<PageState> {
        self.step_counter += 1;
        let step_start = Instant::now();

        info!("Executing action {}: {:?}", self.step_counter, action.action_type);

        let result = match action.action_type {
            ActionType::Navigate => {
                let url = action
                    .url
                    .as_ref()
                    .ok_or_else(|| anyhow!("Navigate action requires url"))?;
                self.driver.navigate(url).await
            }
            ActionType::Click => {
                if let Some(index) = action.index {
                    self.driver.click_by_index(index).await?;
                } else if let Some(ref selector) = action.selector {
                    self.driver.click(selector).await?;
                } else {
                    return Err(anyhow!("Click action requires selector or index"));
                }
                self.driver.get_page_state().await
            }
            ActionType::Fill => {
                let selector = action
                    .selector
                    .as_ref()
                    .ok_or_else(|| anyhow!("Fill action requires selector"))?;
                let value = action
                    .value
                    .as_ref()
                    .ok_or_else(|| anyhow!("Fill action requires value"))?;
                self.driver.fill(selector, value).await?;
                self.driver.get_page_state().await
            }
            ActionType::Back => {
                self.driver.go_back().await?;
                self.driver.get_page_state().await
            }
            ActionType::WaitForApi => {
                let pattern = action
                    .url_pattern
                    .as_ref()
                    .ok_or_else(|| anyhow!("WaitForApi action requires url_pattern"))?;
                let timeout = Duration::from_millis(action.timeout_ms.unwrap_or(10000));
                self.driver.wait_for_request(pattern, timeout).await?;
                self.driver.get_page_state().await
            }
            ActionType::Extract => {
                // For now, just return current state
                // Extraction logic can be added later
                self.driver.get_page_state().await
            }
            ActionType::Scroll | ActionType::Wait | ActionType::Finish => {
                self.driver.get_page_state().await
            }
        };

        let duration_ms = step_start.elapsed().as_millis() as u64;

        // Record execution step
        let step = ExecutionStep {
            step_number: self.step_counter,
            action: action.clone(),
            success: result.is_ok(),
            error: result.as_ref().err().map(|e| e.to_string()),
            resulting_state: result.as_ref().ok().cloned(),
            timestamp: SystemTime::now(),
            duration_ms,
        };

        self.execution_history.push(step);

        result
    }

    /// Get the browser driver (for tool access)
    pub fn driver(&self) -> Arc<BrowserDriver> {
        self.driver.clone()
    }

    /// Get execution history
    pub fn history(&self) -> &[ExecutionStep] {
        &self.execution_history
    }
}

