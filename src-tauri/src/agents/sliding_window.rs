use anyhow::Result;
use chrono::Utc;
use serde_json::{json, Value};

use std::collections::VecDeque;
use std::sync::Arc;
use tauri::{AppHandle, Manager, Emitter};
use tracing::{info}; // Removed warn

use sentinel_db::Database; // Added Database trait
use sentinel_llm::{ChatMessage, LlmClient, LlmConfig};
use crate::agents::context_engineering::tool_digest::condense_text;

/// Configuration for sliding window manager
#[derive(Debug, Clone)]
pub struct SlidingWindowConfig {
    /// Number of messages per segment (trigger threshold)
    pub segment_size: usize,
    /// Number of recent messages to keep fully intact
    pub recent_message_count: usize,
    /// Max number of segment summaries to keep before merging to global
    pub max_segment_summaries: usize,
    /// Max context tokens (dynamically loaded from provider config)
    pub max_context_tokens: usize,
    /// Token allocation ratio for global summary
    pub global_summary_ratio: f64,
    /// Token allocation ratio for segment summaries
    pub segment_summary_ratio: f64,
}

#[derive(Debug, Clone, Copy)]
pub struct SlidingWindowSummaryStats {
    pub global_summary_tokens: usize,
    pub segment_summary_tokens: usize,
    pub segment_count: usize,
}

impl Default for SlidingWindowConfig {
    fn default() -> Self {
        Self {
            segment_size: 20,
            recent_message_count: 20,
            max_segment_summaries: 10,
            max_context_tokens: 128000,
            global_summary_ratio: 0.08,  // 8% for global summary
            segment_summary_ratio: 0.15, // 15% for segment summaries
            // Remaining ~77% for history, but we reserve 30% for system prompt + tools
            // So effective history budget is ~47% of max_context_tokens
        }
    }
}

// 从 sentinel-core 导入模型定义
pub use sentinel_core::models::database::{ConversationSegment, GlobalSummary};

/// Manager for sliding window memory
pub struct SlidingWindowManager {
    app_handle: AppHandle,
    conversation_id: String,
    db: Arc<dyn Database>,
    config: SlidingWindowConfig,
    
    // Memory State
    global_summary: Option<GlobalSummary>,
    segments: VecDeque<ConversationSegment>,
    recent_messages: VecDeque<ChatMessage>,
    
    // Metadata
    total_processed_messages: i32,
}

const SAFE_CONTEXT_RATIO: f64 = 0.85;
const SUMMARY_INPUT_MAX_CHARS: usize = 12_000;

impl SlidingWindowManager {
    /// Initialize manager, ensuring tables exist and loading state
    pub async fn new(
        app_handle: &AppHandle,
        conversation_id: &str,
        config: Option<SlidingWindowConfig>,
    ) -> Result<Self> {
        let db = app_handle.state::<Arc<dyn Database>>().inner().clone();

        // Ensure tables exist
        db.ensure_sliding_window_tables_exist().await?;

        // Load config
        let final_config = config.unwrap_or_default();
        
        // Load state from DB
        let (global_summary, segments) = db.get_sliding_window_summaries(conversation_id).await?;
        
        // Determine where we left off
        let last_summarized_index = segments
            .iter()
            .last()
            .map(|s| s.end_message_index)
            .or_else(|| global_summary.as_ref().map(|g| g.covers_up_to_index))
            .unwrap_or(-1);

        // Load recent messages
        let recent_messages = Self::load_recent_messages(&db, conversation_id, last_summarized_index).await?;
        
        let total_processed_messages = last_summarized_index + 1 + recent_messages.len() as i32;

        Ok(Self {
            app_handle: app_handle.clone(),
            conversation_id: conversation_id.to_string(),
            db,
            config: final_config,
            global_summary,
            segments: segments.into(),
            recent_messages: recent_messages.into(),
            total_processed_messages,
        })
    }


    async fn load_recent_messages(
        db: &Arc<dyn Database>,
        conversation_id: &str,
        after_index: i32,
    ) -> Result<Vec<ChatMessage>> {
        let all_messages = db.get_ai_messages_by_conversation(conversation_id).await?;
        
        // Convert to ChatMessage
        let chat_messages = crate::commands::ai::reconstruct_chat_history(&all_messages);
        
        // Skip already summarized ones
        // Assuming strict ordering: summarized count = after_index + 1
        let skip_count = (after_index + 1) as usize;

        if skip_count >= chat_messages.len() {
            if !chat_messages.is_empty() && after_index >= 0 {
                tracing::warn!(
                    "Sliding window skip_count {} exceeds message count {} for conversation {}, returning full history",
                    skip_count,
                    chat_messages.len(),
                    conversation_id
                );
                return Ok(chat_messages);
            }
            return Ok(Vec::new());
        }

        Ok(chat_messages.into_iter().skip(skip_count).collect())
    }

    /// Add a new message to the recent window (in-memory only, persistence is handled by executor)
    pub fn add_message(&mut self, message: ChatMessage) {
        self.recent_messages.push_back(message);
        self.total_processed_messages += 1;
    }

    /// Build the context for LLM execution
    pub fn build_context(&self, system_prompt: &str) -> Vec<ChatMessage> {
        let mut context = Vec::new();
        
        // 1. System Prompt with Global Context
        let mut full_system_prompt = system_prompt.to_string();
        
        if let Some(global) = &self.global_summary {
            full_system_prompt.push_str("\n\n=== LONG-TERM MEMORY ===\n");
            full_system_prompt.push_str(&global.summary);
        }

        if !self.segments.is_empty() {
             full_system_prompt.push_str("\n\n=== RECENT ACTIVITY SUMMARY ===\n");
             for segment in &self.segments {
                 full_system_prompt.push_str(&format!("- {}\n", segment.summary));
             }
        }

        context.push(ChatMessage {
            role: "system".to_string(),
            content: full_system_prompt,
            tool_calls: None,
            tool_call_id: None,
            reasoning_content: None,
        });

        // 2. Recent Messages
        for msg in &self.recent_messages {
            context.push(msg.clone());
        }

        context
    }

    pub fn summary_stats(&self) -> SlidingWindowSummaryStats {
        let global_summary_tokens = self
            .global_summary
            .as_ref()
            .map(|summary| {
                if summary.summary_tokens > 0 {
                    summary.summary_tokens as usize
                } else {
                    estimate_tokens(&summary.summary)
                }
            })
            .unwrap_or(0);

        let segment_summary_tokens = self
            .segments
            .iter()
            .map(|segment| {
                if segment.summary_tokens > 0 {
                    segment.summary_tokens as usize
                } else {
                    estimate_tokens(&segment.summary)
                }
            })
            .sum();

        SlidingWindowSummaryStats {
            global_summary_tokens,
            segment_summary_tokens,
            segment_count: self.segments.len(),
        }
    }

    /// Check and compress history if needed
    /// Returns true if compression occurred
    pub async fn compress_if_needed(&mut self, llm_config: &LlmConfig) -> Result<bool> {
        // Calculate tokens for all message components
        let recent_tokens: usize = self.recent_messages.iter()
            .map(|m| {
                let mut tokens = estimate_tokens(&m.content);
                if let Some(ref tc) = m.tool_calls {
                    tokens += estimate_tokens(tc);
                }
                if let Some(ref rc) = m.reasoning_content {
                    tokens += estimate_tokens(rc);
                }
                tokens
            })
            .sum();
        
        // Align with builder safe limit ratio and reserve summary allocations
        let history_ratio = SAFE_CONTEXT_RATIO - self.config.global_summary_ratio - self.config.segment_summary_ratio;
        let threshold_tokens = (self.config.max_context_tokens as f64 * history_ratio.max(0.3)) as usize;
        
        let should_segment = self.recent_messages.len() > self.config.recent_message_count 
            || recent_tokens > threshold_tokens;

        if should_segment {
            info!("Triggering sliding window compression. Messages: {}, Tokens: {}/{}", 
                self.recent_messages.len(), recent_tokens, threshold_tokens);
            
            self.create_segment_summary(llm_config).await?;
            
            // After creating a segment, check if we need to merge to global summary
            if self.segments.len() > self.config.max_segment_summaries {
                self.merge_to_global_summary(llm_config).await?;
            }
            
            return Ok(true);
        }

        Ok(false)
    }

    async fn create_segment_summary(&mut self, llm_config: &LlmConfig) -> Result<()> {
        if self.recent_messages.is_empty() {
            return Ok(());
        }

        // Determine how many messages to summarize
        // We keep the most recent 'recent_message_count' / 2 messages to maintain context continuity
        let mut keep_count = (self.config.recent_message_count / 2).min(self.recent_messages.len());
        keep_count = self.adjust_keep_count_for_tool_boundaries(keep_count);
        if self.recent_messages.len() <= keep_count {
            return Ok(());
        }

        let summarize_count = self.recent_messages.len() - keep_count;
        let mut messages_to_summarize = Vec::new();
        
        for _ in 0..summarize_count {
            if let Some(msg) = self.recent_messages.pop_front() {
                messages_to_summarize.push(msg);
            }
        }

        if messages_to_summarize.is_empty() {
            return Ok(());
        }

        // Generate summary
        let summary_text = self.generate_summary(&messages_to_summarize, llm_config).await?;
        let summary_tokens = estimate_tokens(&summary_text) as i32;

        // Calculate indices (based on what we tracked)
        // The start index is what follows the last known index.
        let last_index = self.segments.iter().last().map(|s| s.end_message_index).or_else(|| self.global_summary.as_ref().map(|g| g.covers_up_to_index)).unwrap_or(-1);
        
        let start_index = last_index + 1;
        let end_index = start_index + messages_to_summarize.len() as i32 - 1;

        let segment_index = self.segments.iter().last().map(|s| s.segment_index + 1).unwrap_or(0);

        let segment = ConversationSegment {
            id: uuid::Uuid::new_v4().to_string(),
            conversation_id: self.conversation_id.clone(),
            segment_index,
            start_message_index: start_index,
            end_message_index: end_index,
            summary: summary_text,
            summary_tokens,
            created_at: Utc::now().timestamp(),
        };

        // Save to DB
        self.db.save_conversation_segment(&segment).await?;

        self.segments.push_back(segment.clone());
        
        info!("Created segment summary #{} covering messages {}-{}", segment_index, start_index, end_index);

        let _ = self.app_handle.emit(
            "agent:segment_summary_created",
            &json!({
                "conversation_id": self.conversation_id,
                "segment_index": segment.segment_index,
                "summary": segment.summary,
                "tokens": segment.summary_tokens
            })
        );

        Ok(())
    }

    async fn merge_to_global_summary(&mut self, llm_config: &LlmConfig) -> Result<()> {
        let excess_count = self.segments.len().saturating_sub(self.config.max_segment_summaries / 2); // Merge half
        if excess_count == 0 {
            return Ok(());
        }

        let mut segments_to_merge = Vec::new();
        for _ in 0..excess_count {
            if let Some(seg) = self.segments.pop_front() {
                segments_to_merge.push(seg);
            }
        }

        if segments_to_merge.is_empty() {
            return Ok(());
        }

        let new_covers_up_to = segments_to_merge.last().unwrap().end_message_index;
        
        // Prepare prompt
        let mut prompt = String::new();
        if let Some(global) = &self.global_summary {
            prompt.push_str("Current Global Summary:\n");
            prompt.push_str(&global.summary);
            prompt.push_str("\n\n");
        }
        
        prompt.push_str("New Activity Segments to Merge:\n");
        for seg in &segments_to_merge {
            prompt.push_str(&format!("- {}\n", seg.summary));
        }

        prompt.push_str(
            "\n\nTask: Integrate the new activity segments into the global summary. Maintain a coherent narrative of the user's goals, key decisions, and progress. Remove obsolete details. Preserve exact user-provided literals (URLs, file paths, host:port, identifiers, commands) and include them verbatim in a 'Key User Inputs' section."
        );

        let client = LlmClient::new(llm_config.clone());
        let new_summary_text = client.completion(
            Some("You are a memory consolidation assistant. Merge the conversation logs into a concise global summary."),
            &prompt
        ).await?;

        let new_summary = GlobalSummary {
            id: uuid::Uuid::new_v4().to_string(),
            conversation_id: self.conversation_id.clone(),
            summary: new_summary_text,
            summary_tokens: 0, // Recalc later if needed
            covers_up_to_index: new_covers_up_to,
            updated_at: Utc::now().timestamp(),
        };

        // Save to DB (Upsert)
        self.db.upsert_global_summary(&new_summary).await?;

        self.global_summary = Some(new_summary.clone());
        
        // Delete merged segments from DB
        let ids: Vec<String> = segments_to_merge.iter().map(|s| s.id.clone()).collect();
        self.db.delete_conversation_segments(&ids).await?;
        
        info!("Merged segments into global summary. Covered up to index {}", new_covers_up_to);

        let _ = self.app_handle.emit(
            "agent:global_summary_updated",
            &json!({
                "conversation_id": self.conversation_id,
                "summary": new_summary.summary,
                "tokens": estimate_tokens(&new_summary.summary)
            })
        );

        Ok(())
    }

    async fn generate_summary(&self, messages: &[ChatMessage], llm_config: &LlmConfig) -> Result<String> {
        let mut content = String::new();
        for msg in messages {
            let mut msg_content = msg.content.clone();
            if msg.role == "tool" {
                if let Some(condensed) = condense_tool_output(&msg.content) {
                    msg_content = condensed;
                } else {
                    msg_content = trim_text(&msg.content, 12, 800);
                }
            } else if msg.content.len() > 1200 {
                msg_content = trim_text(&msg.content, 12, 1200);
            }

            content.push_str(&format!("{}: {}\n", msg.role, msg_content));
            if let Some(tool_calls) = &msg.tool_calls {
                content.push_str(&format!("[Tool Calls: {}]\n", tool_calls));
            }
        }

        if content.len() > SUMMARY_INPUT_MAX_CHARS {
            content = condense_text(&content, SUMMARY_INPUT_MAX_CHARS);
        }

        let prompt = format!(
            "Summarize the following conversation segment. Use only facts explicitly present in the messages; do not infer completion or success unless a tool result or assistant message states it. If uncertain, label as Unknown. Focus on task key facts, decisions, and tool results. For shell/interactive_shell, keep command, completion status, and only short output snippets; omit long logs. Preserve exact user-provided literals (URLs, file paths, host:port, identifiers, commands) and include them verbatim in a 'Key User Inputs' section.\n\n{}",
            content
        );

        let client = LlmClient::new(llm_config.clone());
        client.completion(
            Some("You are a conversation summarizer. Create a concise, structured summary of the events."),
            &prompt
        ).await
    }

    fn adjust_keep_count_for_tool_boundaries(&self, keep_count: usize) -> usize {
        if self.recent_messages.is_empty() || keep_count == 0 {
            return keep_count;
        }

        let mut adjusted = keep_count;
        loop {
            let boundary = self.recent_messages.len().saturating_sub(adjusted);
            if boundary >= self.recent_messages.len() {
                break;
            }
            let msg = &self.recent_messages[boundary];
            if msg.role == "tool" {
                if adjusted == 0 {
                    break;
                }
                adjusted = adjusted.saturating_sub(1);
                continue;
            }
            break;
        }

        adjusted
    }

    /// Export full conversation history to formatted string
    pub async fn export_history(&self) -> Result<String> {
        let all_messages = self.db.get_ai_messages_by_conversation(&self.conversation_id).await?;
        
        let mut content = String::new();
        content.push_str(&format!("=== Conversation History: {} ===\n", self.conversation_id));
        content.push_str(&format!("Total Messages: {}\n", all_messages.len()));
        content.push_str(&format!("Exported At: {}\n\n", chrono::Utc::now().format("%Y-%m-%d %H:%M:%S UTC")));
        
        for (idx, msg) in all_messages.iter().enumerate() {
            content.push_str(&format!("--- Message #{} [{} at {}] ---\n", 
                idx + 1,
                msg.role,
                msg.timestamp.format("%Y-%m-%d %H:%M:%S")
            ));
            content.push_str(&msg.content);
            content.push_str("\n\n");
        }
        
        Ok(content)
    }
}

/// Estimate token count for text (improved heuristic)
/// Uses a more conservative estimate to avoid context overflow
fn estimate_tokens(text: &str) -> usize {
    if text.is_empty() {
        return 0;
    }
    let mut total: f64 = 0.0;
    for c in text.chars() {
        if c.is_ascii() {
            // More conservative: ~0.4 tokens per ASCII char
            total += 0.4;
        } else {
            // CJK and other non-ASCII: ~1.6 tokens per char
            total += 1.6;
        }
    }
    // Add 20% safety margin
    (total * 1.2).ceil() as usize
}

fn condense_tool_output(raw: &str) -> Option<String> {
    let value: Value = serde_json::from_str(raw).ok()?;
    let obj = value.as_object()?;

    // shell tool output
    if obj.contains_key("command") && obj.contains_key("stdout") && obj.contains_key("stderr") {
        let command = obj.get("command").and_then(|v| v.as_str()).unwrap_or("");
        let exit_code = obj.get("exit_code").and_then(|v| v.as_i64()).map(|v| v.to_string()).unwrap_or_else(|| "unknown".to_string());
        let completed = obj.get("completed").and_then(|v| v.as_bool()).unwrap_or(true);
        let output_stored = obj.get("output_stored").and_then(|v| v.as_bool()).unwrap_or(false);
        let stdout = obj.get("stdout").and_then(|v| v.as_str()).unwrap_or("");
        let stderr = obj.get("stderr").and_then(|v| v.as_str()).unwrap_or("");
        let stdout_snip = trim_text(stdout, 8, 500);
        let stderr_snip = trim_text(stderr, 6, 400);
        return Some(format!(
            "shell: command=\"{}\" exit_code={} completed={} output_stored={} stdout_snip=\"{}\" stderr_snip=\"{}\"",
            command, exit_code, completed, output_stored, stdout_snip, stderr_snip
        ));
    }

    // interactive_shell output
    if obj.contains_key("session_id") && obj.contains_key("output") && obj.contains_key("completed") {
        let command = obj.get("command").and_then(|v| v.as_str()).unwrap_or("");
        let completed = obj.get("completed").and_then(|v| v.as_bool()).unwrap_or(false);
        let truncated = obj.get("truncated").and_then(|v| v.as_bool()).unwrap_or(false);
        let output = obj.get("output").and_then(|v| v.as_str()).unwrap_or("");
        let output_snip = trim_text(output, 10, 600);
        return Some(format!(
            "interactive_shell: command=\"{}\" completed={} truncated={} output_snip=\"{}\"",
            command, completed, truncated, output_snip
        ));
    }

    None
}

fn trim_text(text: &str, max_lines: usize, max_chars: usize) -> String {
    if text.is_empty() {
        return String::new();
    }

    let mut lines = text.lines().take(max_lines).collect::<Vec<_>>().join("\n");
    if lines.len() > max_chars {
        let mut boundary = max_chars.min(lines.len());
        while boundary > 0 && !lines.is_char_boundary(boundary) {
            boundary -= 1;
        }
        lines.truncate(boundary);
    }

    if text.lines().count() > max_lines || text.len() > max_chars {
        lines.push_str(" ...[truncated]");
    }

    lines
}
