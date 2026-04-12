//! Context packet definitions and shared helpers.

use sentinel_llm::ChatMessage;
use serde::{Deserialize, Serialize};

use crate::agents::context_engineering::tool_digest::ToolDigest;
use crate::agents::context_engineering::ContextMessageLayout;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ContextSection {
    System,
    OrchestratorContext,
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
pub struct RetrievedMemorySection {
    pub title: String,
    pub items: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ContextPacket {
    pub system_instructions: String,
    pub orchestrator_context: String,
    pub run_state: String,
    pub window_messages: Vec<ChatMessage>,
    pub retrieved_memories: Vec<String>,
    pub retrieved_memory_sections: Vec<RetrievedMemorySection>,
    pub tool_digests: Vec<ToolDigestEntry>,
}

impl ContextPacket {
    pub fn new(system_instructions: String) -> Self {
        Self {
            system_instructions,
            orchestrator_context: String::new(),
            run_state: String::new(),
            window_messages: Vec::new(),
            retrieved_memories: Vec::new(),
            retrieved_memory_sections: Vec::new(),
            tool_digests: Vec::new(),
        }
    }

    pub fn render_system_prompt(&self) -> String {
        self.system_instructions.clone()
    }

    pub fn render_orchestrator_context(&self) -> String {
        let mut prompt = String::new();
        if !self.run_state.trim().is_empty() {
            prompt.push_str("[RunState]\n");
            prompt.push_str(self.run_state.trim());
        }

        let rendered_retrieval = self.render_retrieved_memory_context();
        if !rendered_retrieval.is_empty() {
            if !prompt.is_empty() {
                prompt.push_str("\n\n");
            }
            prompt.push_str(&rendered_retrieval);
        }

        if !self.tool_digests.is_empty() {
            if !prompt.is_empty() {
                prompt.push_str("\n\n");
            }
            prompt.push_str("[Recent Tool Digests]\n");
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

        prompt.trim().to_string()
    }

    pub fn render_context_messages(&self, layout: ContextMessageLayout) -> Vec<ChatMessage> {
        match layout {
            ContextMessageLayout::SingleUserMessage => {
                let rendered = self.render_orchestrator_context();
                if rendered.trim().is_empty() {
                    Vec::new()
                } else {
                    vec![ChatMessage::user(rendered)]
                }
            }
            ContextMessageLayout::SplitUserMessages => {
                let mut messages = Vec::new();

                if !self.run_state.trim().is_empty() {
                    messages.push(ChatMessage::user(format!(
                        "[RunState]\n{}",
                        self.run_state.trim()
                    )));
                }

                let rendered_retrieval = self.render_retrieved_memory_context();
                if !rendered_retrieval.is_empty() {
                    let body = rendered_retrieval;
                    messages.push(ChatMessage::user(body.trim().to_string()));
                }

                if !self.tool_digests.is_empty() {
                    let mut body = String::from("[Recent Tool Digests]\n");
                    for digest in &self.tool_digests {
                        if let Some(artifact_id) = digest.artifact_id.as_ref() {
                            body.push_str(&format!(
                                "- [{}] {}: {} (artifact_id: {})\n",
                                digest.status, digest.tool_name, digest.summary, artifact_id
                            ));
                        } else {
                            body.push_str(&format!(
                                "- [{}] {}: {}\n",
                                digest.status, digest.tool_name, digest.summary
                            ));
                        }
                    }
                    messages.push(ChatMessage::user(body.trim().to_string()));
                }

                messages
            }
        }
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

    pub fn render_retrieved_memory_context(&self) -> String {
        if !self.retrieved_memory_sections.is_empty() {
            let mut body = String::from("[RetrievedMemory]\n");
            for section in &self.retrieved_memory_sections {
                if section.items.is_empty() {
                    continue;
                }
                body.push_str(section.title.trim());
                body.push_str(":\n");
                for item in &section.items {
                    body.push_str("- ");
                    body.push_str(item.trim());
                    body.push('\n');
                }
            }
            return body.trim().to_string();
        }

        if self.retrieved_memories.is_empty() {
            return String::new();
        }

        let mut body = String::from("[RetrievedMemory]\n");
        for item in &self.retrieved_memories {
            body.push_str("- ");
            body.push_str(item.trim());
            body.push('\n');
        }
        body.trim().to_string()
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
