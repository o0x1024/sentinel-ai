//! Shared token estimation utilities.

use sentinel_llm::ChatMessage;

pub const SYSTEM_MESSAGE_OVERHEAD_TOKENS: usize = 10;
pub const MESSAGE_OVERHEAD_TOKENS: usize = 12;
pub const TOOL_CALLS_OVERHEAD_TOKENS: usize = 16;

/// Estimate token count for a ChatMessage including all fields.
pub fn estimate_message_tokens(msg: &ChatMessage) -> usize {
    let mut tokens = estimate_tokens(&msg.content);
    tokens += MESSAGE_OVERHEAD_TOKENS;

    if let Some(ref tc) = msg.tool_calls {
        tokens += estimate_tokens(tc);
        tokens += TOOL_CALLS_OVERHEAD_TOKENS;
    }
    if let Some(ref rc) = msg.reasoning_content {
        tokens += estimate_tokens(rc);
    }
    if let Some(ref tid) = msg.tool_call_id {
        tokens += estimate_tokens(tid);
    }

    tokens
}

/// Estimate token count for text using a conservative character-based heuristic.
///
/// Uses ~0.4 tokens per ASCII char and ~1.6 tokens per CJK/non-ASCII char,
/// plus a 20% safety margin.
pub fn estimate_tokens(text: &str) -> usize {
    if text.is_empty() {
        return 0;
    }
    let mut total: f64 = 0.0;
    for c in text.chars() {
        if c.is_ascii() {
            total += 0.4;
        } else {
            total += 1.6;
        }
    }
    (total * 1.2).ceil() as usize
}
