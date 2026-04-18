//! Context packet definitions and shared helpers.

use sentinel_llm::ChatMessage;
use serde::{Deserialize, Serialize};
use serde_json::{Map, Value};

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
    #[serde(default)]
    pub review_hint: Option<String>,
    #[serde(default)]
    pub verification_status: Option<String>,
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
            prompt.push_str(&render_tool_digest_section(&self.tool_digests));
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
                    let body = render_tool_digest_section(&self.tool_digests);
                    messages.push(ChatMessage::user(body.trim().to_string()));
                }

                messages
            }
        }
    }

    pub fn set_tool_digests(&mut self, digests: &[ToolDigest]) {
        self.tool_digests = build_tool_digest_entries(digests);
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

fn render_tool_digest_section(digests: &[ToolDigestEntry]) -> String {
    let mut body = String::from("[Recent Tool Digests]\n");
    if let Some(guidance) = build_file_change_self_check_guidance(digests) {
        body.push_str(guidance.trim());
        body.push('\n');
    }
    for digest in digests {
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
        if let Some(review_hint) = digest.review_hint.as_ref() {
            body.push_str("  review: ");
            body.push_str(review_hint.trim());
            body.push('\n');
        }
    }
    body
}

fn build_file_change_self_check_guidance(digests: &[ToolDigestEntry]) -> Option<String> {
    let pending_targets = digests
        .iter()
        .filter(|digest| matches!(digest.tool_name.as_str(), "file_edit" | "file_write"))
        .filter(|digest| digest.verification_status.as_deref() != Some("verified"))
        .filter_map(|digest| digest.review_hint.as_deref())
        .take(3)
        .collect::<Vec<_>>();
    if !pending_targets.is_empty() {
        let mut guidance = String::from(
            "Self-check: some recent file modifications are still unverified. Before another edit or the final answer, use `file_read` to re-read the changed lines and confirm the written content hash.",
        );
        for target in pending_targets {
            guidance.push_str("\n- pending: ");
            guidance.push_str(target.trim());
        }
        return Some(guidance);
    }

    let verified_targets = digests
        .iter()
        .filter(|digest| matches!(digest.tool_name.as_str(), "file_edit" | "file_write"))
        .filter(|digest| digest.verification_status.as_deref() == Some("verified"))
        .filter_map(|digest| digest.review_hint.as_deref())
        .take(2)
        .collect::<Vec<_>>();
    if verified_targets.is_empty() {
        return None;
    }

    let mut guidance = String::from(
        "Self-check: recent file modifications were already read back and verified against the latest content hash.",
    );
    for target in verified_targets {
        guidance.push_str("\n- verified: ");
        guidance.push_str(target.trim());
    }
    Some(guidance)
}

fn build_tool_digest_entries(digests: &[ToolDigest]) -> Vec<ToolDigestEntry> {
    let verification_states = compute_file_change_verification_states(digests);
    digests
        .iter()
        .enumerate()
        .map(|(index, digest)| ToolDigestEntry {
            status: digest.status.clone(),
            tool_name: digest.tool_name.clone(),
            summary: digest.summary.clone(),
            artifact_id: digest.artifact_id.clone(),
            review_hint: build_review_hint(
                digest,
                verification_states.get(index).copied().flatten(),
            ),
            verification_status: verification_states
                .get(index)
                .copied()
                .flatten()
                .map(str::to_string),
        })
        .collect()
}

fn compute_file_change_verification_states(digests: &[ToolDigest]) -> Vec<Option<&'static str>> {
    let mut entries = vec![None; digests.len()];

    for (index, digest) in digests.iter().enumerate() {
        if !matches!(digest.tool_name.as_str(), "file_edit" | "file_write") || digest.status != "ok"
        {
            continue;
        }

        let Some((artifact_id, content_hash)) = digest_artifact_and_hash(digest) else {
            entries[index] = Some("pending");
            continue;
        };

        let verified = digests[index + 1..].iter().any(|later_digest| {
            if later_digest.tool_name != "file_read" || later_digest.status != "ok" {
                return false;
            }
            match digest_artifact_and_hash(later_digest) {
                Some((read_artifact_id, read_hash)) => {
                    read_artifact_id == artifact_id && read_hash == content_hash
                }
                None => false,
            }
        });
        entries[index] = Some(if verified { "verified" } else { "pending" });
    }

    entries
}

fn build_review_hint(
    digest: &ToolDigest,
    verification_status: Option<&'static str>,
) -> Option<String> {
    let metadata = digest.metadata.as_ref()?.as_object()?;
    let change_summary = metadata.get("change_summary")?.as_object()?;

    let first_changed_line = change_summary
        .get("first_changed_line")
        .and_then(|value| value.as_u64());
    let changed_line_count = change_summary
        .get("changed_line_count")
        .and_then(|value| value.as_u64());
    let added_line_count = change_summary
        .get("added_line_count")
        .and_then(|value| value.as_u64());
    let removed_line_count = change_summary
        .get("removed_line_count")
        .and_then(|value| value.as_u64());
    let after_preview = change_summary
        .get("after_preview")
        .and_then(|value| value.as_str())
        .map(str::trim)
        .filter(|value| !value.is_empty());

    let mut parts = Vec::new();
    if let Some(line) = first_changed_line {
        if let Some(count) = changed_line_count {
            parts.push(format!(
                "inspect around line {} ({} changed lines)",
                line, count
            ));
        } else {
            parts.push(format!("inspect around line {}", line));
        }
    }
    if added_line_count.is_some() || removed_line_count.is_some() {
        parts.push(format!(
            "line delta +{} / -{}",
            added_line_count.unwrap_or(0),
            removed_line_count.unwrap_or(0)
        ));
    }
    if let Some(preview) = after_preview {
        let preview = if preview.chars().count() > 100 {
            let head: String = preview.chars().take(100).collect();
            format!("{}...", head)
        } else {
            preview.to_string()
        };
        parts.push(format!("verify snippet `{}`", preview));
    }
    match verification_status {
        Some("verified") => {
            parts.push("content hash verified by a later `file_read`".to_string());
        }
        Some("pending") => {
            parts.push("pending readback verification via `file_read`".to_string());
        }
        _ => {}
    }

    if parts.is_empty() {
        None
    } else {
        Some(parts.join(" | "))
    }
}

fn digest_artifact_and_hash(digest: &ToolDigest) -> Option<(String, String)> {
    let metadata = digest.metadata.as_ref()?.as_object()?;
    let content_hash = metadata
        .get("content_hash")
        .and_then(|value| value.as_str())?
        .to_string();
    let artifact_id =
        extract_artifact_id_from_metadata(metadata).or_else(|| digest.artifact_id.clone())?;
    Some((artifact_id, content_hash))
}

fn extract_artifact_id_from_metadata(metadata: &Map<String, Value>) -> Option<String> {
    metadata
        .get("artifact_read")
        .and_then(|value| value.get("artifact_id"))
        .and_then(|value| value.as_str())
        .map(str::to_string)
        .or_else(|| {
            metadata
                .get("stored_artifacts")
                .and_then(|value| value.as_array())
                .and_then(|items| items.first())
                .and_then(|item| item.get("path"))
                .and_then(|value| value.as_str())
                .map(str::to_string)
        })
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

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn digest(tool_name: &str, metadata: Value) -> ToolDigest {
        ToolDigest {
            tool_name: tool_name.to_string(),
            status: "ok".to_string(),
            summary: format!("{} summary", tool_name),
            artifact_id: None,
            artifact_kind: None,
            preview_snippets: Vec::new(),
            metadata: Some(metadata),
            created_at_ms: 0,
        }
    }

    #[test]
    fn tool_digest_entries_mark_pending_and_verified_file_changes() {
        let digests = vec![
            digest(
                "file_write",
                json!({
                    "content_hash": "hash-a",
                    "stored_artifacts": [{"path":"/tmp/a.txt"}],
                    "change_summary": {"first_changed_line": 1, "changed_line_count": 2, "after_preview": "alpha"}
                }),
            ),
            digest(
                "file_read",
                json!({
                    "content_hash": "hash-a",
                    "artifact_read": {"artifact_id": "/tmp/a.txt"}
                }),
            ),
            digest(
                "file_edit",
                json!({
                    "content_hash": "hash-b",
                    "stored_artifacts": [{"path":"/tmp/b.txt"}],
                    "change_summary": {"first_changed_line": 4, "changed_line_count": 1, "after_preview": "beta"}
                }),
            ),
        ];

        let entries = build_tool_digest_entries(&digests);

        assert_eq!(entries[0].verification_status.as_deref(), Some("verified"));
        assert_eq!(entries[2].verification_status.as_deref(), Some("pending"));
        assert!(entries[0]
            .review_hint
            .as_deref()
            .unwrap_or_default()
            .contains("content hash verified"));
        assert!(entries[2]
            .review_hint
            .as_deref()
            .unwrap_or_default()
            .contains("pending readback verification"));
    }

    #[test]
    fn file_change_guidance_prefers_pending_changes_over_verified_ones() {
        let entries = vec![
            ToolDigestEntry {
                status: "ok".to_string(),
                tool_name: "file_write".to_string(),
                summary: "write".to_string(),
                artifact_id: Some("/tmp/a.txt".to_string()),
                review_hint: Some("inspect around line 1".to_string()),
                verification_status: Some("verified".to_string()),
            },
            ToolDigestEntry {
                status: "ok".to_string(),
                tool_name: "file_edit".to_string(),
                summary: "edit".to_string(),
                artifact_id: Some("/tmp/b.txt".to_string()),
                review_hint: Some("inspect around line 4".to_string()),
                verification_status: Some("pending".to_string()),
            },
        ];

        let guidance = build_file_change_self_check_guidance(&entries).expect("guidance");

        assert!(guidance.contains("still unverified"));
        assert!(guidance.contains("pending: inspect around line 4"));
        assert!(!guidance.contains("verified: inspect around line 1"));
    }
}
