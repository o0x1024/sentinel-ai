//! Context engine modes and policy selection.

use serde::{Deserialize, Serialize};

use super::policy::ContextPolicy;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "kebab-case")]
pub enum ContextEngineMode {
    #[default]
    ClaudeLike,
    CodexLike,
}

impl ContextEngineMode {
    pub fn from_str(value: &str) -> Option<Self> {
        match value.trim().to_ascii_lowercase().as_str() {
            "claude-like" | "claudelike" | "claude" => Some(Self::ClaudeLike),
            "codex-like" | "codexlike" | "codex" => Some(Self::CodexLike),
            _ => None,
        }
    }
}

pub fn resolve_context_policy(
    base: Option<ContextPolicy>,
    mode: ContextEngineMode,
) -> ContextPolicy {
    match mode {
        ContextEngineMode::ClaudeLike => base.unwrap_or_else(ContextPolicy::claude_like),
        ContextEngineMode::CodexLike => {
            let mut policy = base.unwrap_or_else(ContextPolicy::codex_like);
            policy.include_context_storage = false;
            policy.include_task_mainline = true;
            policy.include_run_state = true;
            policy.run_state_max_digests = policy.run_state_max_digests.min(4);
            policy.run_state_max_chars = policy.run_state_max_chars.min(1800);
            policy.layer_max_chars = policy.layer_max_chars.min(10000);
            policy
        }
    }
}
