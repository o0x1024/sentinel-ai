use super::driver::ExplorerDriver;
use super::prompt::SYSTEM_PROMPT;
use crate::engines::LlmClient;
use crate::engines::test_explorer_v1::types::ActionType;
use anyhow::{anyhow, Result};
use log::{info, error, debug};
use serde_json::Value;

pub struct TestExplorerEngine {
    driver: ExplorerDriver,
    llm_client: LlmClient,
    max_steps: usize,
}

impl TestExplorerEngine {
    pub fn new(llm_client: LlmClient) -> Self {
        Self {
            driver: ExplorerDriver::new(),
            llm_client,
            max_steps: 20,
        }
    }

    pub async fn run(&self, initial_url: &str, goal: &str) -> Result<String> {
        info!("Starting TestExplorerEngine task: {}", goal);
        
        // 1. Navigate to initial URL
        self.driver.navigate(initial_url).await?;
        
        let mut steps_taken = 0;
        
        while steps_taken < self.max_steps {
            steps_taken += 1;
            info!("Step {}/{}", steps_taken, self.max_steps);
            
            // 2. Observe State
            let state = self.driver.get_state().await?;
            debug!("Current URL: {}", state.url);
            
            // 3. Construct Prompt
            let user_prompt = format!(
                "Goal: {}\n\nCurrent URL: {}\nTitle: {}\n\nContent Preview:\n{}\n\nInteractive Elements:\n{}", 
                goal,
                state.url,
                state.title,
                self.truncate_content(&state.content, 2000),
                serde_json::to_string_pretty(&state.interactive_elements).unwrap_or_default()
            );

            let full_prompt = format!("{}\n\n{}", SYSTEM_PROMPT, user_prompt);
           let response_text = self.llm_client.completion(None, &full_prompt).await?;
            
            // 5. Parse Action
            let action_json = self.parse_json(response_text)?;
            info!("Agent decided: {:?}", action_json);
            
            let action_type = action_json["action"].as_str().unwrap_or("wait");
            
            // 6. Execute Action
            match action_type {
                "navigate" => {
                    if let Some(url) = action_json["url"].as_str() {
                        self.driver.navigate(url).await?;
                    }
                }
                "click" => {
                    if let Some(idx) = action_json["index"].as_u64() {
                        self.driver.click(idx as usize).await?;
                    }
                }
                "type" => {
                    if let Some(idx) = action_json["index"].as_u64() {
                        if let Some(val) = action_json["value"].as_str() {
                            self.driver.type_text(idx as usize, val).await?;
                        }
                    }
                }
                "scroll" => {
                    let dir = action_json["value"].as_str().unwrap_or("down");
                    self.driver.scroll(dir).await?;
                }
                "back" => {
                    self.driver.go_back().await?;
                }
                "finish" => {
                    let value = action_json["value"].as_str().unwrap_or("Done");
                    return Ok(value.to_string());
                }
                "extract" => {
                     // Just log or return extracted data?
                     // For now, let's treat it as a success/info point but continue if not explicit finish?
                     // Or maybe extract implies finishing the extraction task.
                     let value = action_json["value"].as_str().unwrap_or("Extracted data");
                     return Ok(value.to_string());
                }
                _ => {
                    // Wait or unknown
                    // Proceed to next loop
                }
            }
            
            // Short pause
            tokio::time::sleep(std::time::Duration::from_secs(2)).await;
        }

        Err(anyhow!("Max steps reached without completion"))
    }
    
    fn truncate_content(&self, content: &str, max_len: usize) -> String {
        if content.len() > max_len {
            format!("{}...", &content[..max_len])
        } else {
            content.to_string()
        }
    }

    fn parse_json(&self, text: &str) -> Result<Value> {
        // Strip markdown code blocks if present
        let clean_text = text.trim();
        let clean_text = if clean_text.starts_with("```json") {
             clean_text.strip_prefix("```json").unwrap_or(clean_text)
                .strip_suffix("```").unwrap_or(clean_text)
                .trim()
        } else if clean_text.starts_with("```") {
             clean_text.strip_prefix("```").unwrap_or(clean_text)
                .strip_suffix("```").unwrap_or(clean_text)
                .trim()
        } else {
            clean_text
        };
        
        serde_json::from_str(clean_text).map_err(|e| anyhow!("Failed to parse JSON: {}", e))
    }
}
