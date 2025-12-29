//! Task planner for complex exploration scenarios

use anyhow::{anyhow, Result};
use sentinel_llm::{ChatMessage, LlmClient};
use serde_json::json;
use tracing::{debug, info};

use super::types::{Action, ActionType, PageState};

/// Task planner for decomposing complex goals
pub struct TaskPlanner {
    llm_client: LlmClient,
}

impl TaskPlanner {
    /// Create a new task planner
    pub fn new(llm_client: LlmClient) -> Self {
        Self { llm_client }
    }

    /// Decompose a user goal into a sequence of actions
    pub async fn decompose_task(
        &self,
        user_goal: &str,
        current_state: &PageState,
    ) -> Result<Vec<Action>> {
        info!("Decomposing task: {}", user_goal);

        let system_prompt = r##"You are a web automation planner. Your task is to break down user goals into a sequence of browser actions.

Available actions:
- Navigate: Go to a URL
- Click: Click an element (by selector or index)
- Fill: Fill an input field
- Extract: Extract structured data
- WaitForApi: Wait for an API request
- Back: Go back in history
- Finish: Complete the task

Respond with a JSON array of actions. Each action should have:
{
  "action_type": "Navigate|Click|Fill|Extract|WaitForApi|Back|Finish",
  "selector": "CSS selector (optional)",
  "index": element_index (optional),
  "value": "value for Fill action (optional)",
  "url": "URL for Navigate action (optional)",
  "url_pattern": "pattern for WaitForApi (optional)",
  "timeout_ms": timeout_in_ms (optional)
}

Example:
[
  {"action_type": "Navigate", "url": "https://example.com"},
  {"action_type": "Click", "selector": "#login-button"},
  {"action_type": "Fill", "selector": "#username", "value": "user@example.com"},
  {"action_type": "Click", "index": 5},
  {"action_type": "Finish"}
]

Be concise and efficient. Only include necessary steps."##;

        let user_prompt = format!(
            r#"User Goal: {}

Current Page State:
- URL: {}
- Title: {}
- Visible Text (first 500 chars): {}
- Interactive Elements: {} elements found

Plan the necessary actions to achieve the user's goal. Consider the current page state and available elements."#,
            user_goal,
            current_state.url,
            current_state.title,
            current_state
                .visible_text
                .chars()
                .take(500)
                .collect::<String>(),
            current_state.interactive_elements.len()
        );

        let response = self
            .llm_client
            .chat(Some(system_prompt), &user_prompt, &[], None)
            .await
            .map_err(|e| anyhow!("LLM call failed: {}", e))?;

        debug!("LLM response: {}", response);

        // Extract JSON from response (handle markdown code blocks)
        let json_str = self.extract_json(&response)?;

        // Parse actions
        let actions: Vec<Action> = serde_json::from_str(&json_str)
            .map_err(|e| anyhow!("Failed to parse actions: {}", e))?;

        info!("Decomposed into {} actions", actions.len());
        Ok(actions)
    }

    /// Replan after a failed action
    pub async fn replan(
        &self,
        failed_action: &Action,
        error: &str,
        current_state: &PageState,
    ) -> Result<Vec<Action>> {
        info!("Replanning after failed action");

        let system_prompt = r##"You are a web automation recovery planner. An action failed and you need to provide alternative actions to recover and continue.

Respond with a JSON array of recovery actions."##;

        let user_prompt = format!(
            r#"Failed Action: {:?}
Error: {}

Current Page State:
- URL: {}
- Title: {}
- Interactive Elements: {} elements

Provide alternative actions to recover from this failure and continue toward the goal."#,
            failed_action,
            error,
            current_state.url,
            current_state.title,
            current_state.interactive_elements.len()
        );

        let response = self
            .llm_client
            .chat(Some(system_prompt), &user_prompt, &[], None)
            .await
            .map_err(|e| anyhow!("LLM call failed: {}", e))?;

        let json_str = self.extract_json(&response)?;
        let actions: Vec<Action> = serde_json::from_str(&json_str)
            .map_err(|e| anyhow!("Failed to parse actions: {}", e))?;

        info!("Replanned with {} actions", actions.len());
        Ok(actions)
    }

    /// Extract JSON from LLM response (handle markdown code blocks)
    fn extract_json(&self, response: &str) -> Result<String> {
        let trimmed = response.trim();

        // Try to find JSON array in markdown code block
        if let Some(start) = trimmed.find("```json") {
            if let Some(end) = trimmed[start..].find("```") {
                let json_block = &trimmed[start + 7..start + end];
                return Ok(json_block.trim().to_string());
            }
        }

        // Try to find JSON array directly
        if let Some(start) = trimmed.find('[') {
            if let Some(end) = trimmed.rfind(']') {
                return Ok(trimmed[start..=end].to_string());
            }
        }

        Err(anyhow!("No valid JSON found in response"))
    }

    /// Suggest next action based on current state (single-step planning)
    pub async fn suggest_next_action(&self, current_state: &PageState) -> Result<Action> {
        let system_prompt = r##"You are a web automation assistant. Based on the current page state, suggest the next logical action.

Respond with a single JSON action object."##;

        let user_prompt = format!(
            r#"Current Page State:
- URL: {}
- Title: {}
- Visible Text (first 500 chars): {}
- Interactive Elements: {} elements

Suggest the next action to explore this page effectively."#,
            current_state.url,
            current_state.title,
            current_state
                .visible_text
                .chars()
                .take(500)
                .collect::<String>(),
            current_state.interactive_elements.len()
        );

        let response = self
            .llm_client
            .chat(Some(system_prompt), &user_prompt, &[], None)
            .await
            .map_err(|e| anyhow!("LLM call failed: {}", e))?;

        let json_str = self.extract_json(&response)?;
        let action: Action = serde_json::from_str(&json_str)
            .map_err(|e| anyhow!("Failed to parse action: {}", e))?;

        Ok(action)
    }
}

