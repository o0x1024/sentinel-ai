//! Model-visible tool result size limits.
//!
//! Aligned with Claude Code `DEFAULT_MAX_RESULT_SIZE_CHARS` (50_000) and the
//! Sentinel output storage threshold default.

/// Max serialized tool result chars sent to the model before microcompact.
pub const TOOL_RESULT_MAX_CHARS: usize = 50_000;

/// Max chars per JSON string field during in-value compaction.
pub const TOOL_RESULT_FIELD_MAX_CHARS: usize = 16_000;

/// Max array items kept during in-value compaction.
pub const TOOL_RESULT_ARRAY_MAX_ITEMS: usize = 20;

/// Plain-text preview budget when JSON compaction still exceeds the max.
pub const TOOL_RESULT_PLAIN_PREVIEW_CHARS: usize = 32_000;

/// Head lines considered when building plain-text previews.
pub const TOOL_RESULT_PREVIEW_HEAD_LINES: usize = 80;

/// Slop allowed above [`TOOL_RESULT_MAX_CHARS`] in compacted envelopes.
#[allow(dead_code)] // used by executor tests for assertion bounds
pub const TOOL_RESULT_COMPACT_SLACK_CHARS: usize = 2_000;
