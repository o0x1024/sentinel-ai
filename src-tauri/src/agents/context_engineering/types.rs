//! Context packet definitions and shared helpers.

use sentinel_llm::ChatMessage;
use serde::{Deserialize, Serialize};

use crate::agents::context_engineering::tool_digest::ToolDigest;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ContextSection {
    System,
    RunState,
    Window,
    Retrieval,
    ToolDigest,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolDigestEntry {
    pub status: String,
    pub tool_name: String,
    pub summary: String,
    pub artifact_id: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ContextPacket {
    pub system_instructions: String,
    pub run_state: String,
    pub window_messages: Vec<ChatMessage>,
    pub retrieved_memories: Vec<String>,
    pub tool_digests: Vec<ToolDigestEntry>,
}

impl ContextPacket {
    pub fn new(system_instructions: String) -> Self {
        Self {
            system_instructions,
            run_state: String::new(),
            window_messages: Vec::new(),
            retrieved_memories: Vec::new(),
            tool_digests: Vec::new(),
        }
    }

    pub fn render_system_prompt(&self) -> String {
        let mut prompt = self.system_instructions.clone();
        if !self.run_state.trim().is_empty() {
            prompt.push_str("\n\n[RunState]\n");
            prompt.push_str(self.run_state.trim());
        }

        if !self.retrieved_memories.is_empty() {
            prompt.push_str("\n\n[RetrievedMemory]\n");
            for item in &self.retrieved_memories {
                prompt.push_str("- ");
                prompt.push_str(item.trim());
                prompt.push('\n');
            }
        }

        if !self.tool_digests.is_empty() {
            prompt.push_str("\n\n[Recent Tool Digests]\n");
            for digest in &self.tool_digests {
                if let Some(artifact_id) = digest.artifact_id.as_ref() {
                    prompt.push_str(&format!(
                        "- [{}] {}: {} (artifact_id: {})\n",
                        digest.status, digest.tool_name, digest.summary, artifact_id
                    ));
                } else {
                    prompt.push_str(&format!(
                        "- [{}] {}: {}\n",
                        digest.status, digest.tool_name, digest.summary
                    ));
                }
            }
        }

        prompt
    }

    pub fn set_tool_digests(&mut self, digests: &[ToolDigest]) {
        self.tool_digests = digests
            .iter()
            .map(|digest| ToolDigestEntry {
                status: digest.status.clone(),
                tool_name: digest.tool_name.clone(),
                summary: digest.summary.clone(),
                artifact_id: digest.artifact_id.clone(),
            })
            .collect();
    }
}

pub fn trim_history_preserve_tool_pairs(
    history: &[ChatMessage],
    mut history_tokens: usize,
    available_for_history: usize,
    estimate_message_tokens: impl Fn(&ChatMessage) -> usize,
) -> Vec<ChatMessage> {
    if history_tokens <= available_for_history {
        return history.to_vec();
    }

    let mut trimmed = history.to_vec();
    while history_tokens > available_for_history && !trimmed.is_empty() {
        // Assistant with tool_calls followed by one or more tool responses → remove as group
        if trimmed[0].role == "assistant" && trimmed[0].tool_calls.is_some() {
            let mut tool_count = 0;
            while tool_count + 1 < trimmed.len() && trimmed[tool_count + 1].role == "tool" {
                tool_count += 1;
            }
            if tool_count > 0 {
                for i in 0..=tool_count {
                    history_tokens =
                        history_tokens.saturating_sub(estimate_message_tokens(&trimmed[i]));
                }
                trimmed.drain(0..=tool_count);
                continue;
            }
        }

        // Orphaned tool message at front → remove it
        if trimmed[0].role == "tool" {
            history_tokens = history_tokens.saturating_sub(estimate_message_tokens(&trimmed[0]));
            trimmed.remove(0);
            continue;
        }

        // Regular message (user / assistant without tool_calls)
        history_tokens = history_tokens.saturating_sub(estimate_message_tokens(&trimmed[0]));
        trimmed.remove(0);
    }

    trimmed
}
