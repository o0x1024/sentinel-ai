//! Tenth Man Tool - LLM-callable adversarial review tool
//!
//! Allows the LLM to proactively request critical review of its plans and conclusions.
//! Note: This tool is just the definition. The actual execution logic is in src-tauri/src/agents/tenth_man_executor.rs

use rig::tool::Tool;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

/// Type alias for the actual executor function (provided by main crate)
pub type TenthManExecutorFn = std::sync::Arc<
    dyn Fn(
            TenthManToolArgs,
        ) -> std::pin::Pin<
            Box<
                dyn std::future::Future<Output = Result<TenthManToolOutput, TenthManToolError>>
                    + Send,
            >,
        > + Send
        + Sync,
>;

/// Global executor storage (set by main crate at runtime)
static TENTH_MAN_EXECUTOR: once_cell::sync::OnceCell<TenthManExecutorFn> =
    once_cell::sync::OnceCell::new();

/// Set the executor function (called by main crate during initialization)
pub fn set_tenth_man_executor(executor: TenthManExecutorFn) {
    let _ = TENTH_MAN_EXECUTOR.set(executor);
}

/// Get the executor function
fn get_executor() -> Result<&'static TenthManExecutorFn, TenthManToolError> {
    TENTH_MAN_EXECUTOR.get().ok_or_else(|| {
        TenthManToolError::InternalError("Tenth Man executor not initialized".to_string())
    })
}

/// Review mode for Tenth Man
#[derive(Debug, Clone, Deserialize, Serialize, JsonSchema)]
#[serde(tag = "mode", rename_all = "snake_case")]
#[derive(Default)]
pub enum ReviewMode {
    /// Review complete history (using sliding window summarization)
    #[default]
    FullHistory,
    /// Review recent N messages
    RecentMessages {
        #[schemars(description = "Number of recent messages to review")]
        count: usize,
    },
    /// Review specific content (backward compatible)
    SpecificContent {
        #[schemars(description = "Specific content to review")]
        content: String,
    },
}

/// Tenth Man tool arguments
#[derive(Debug, Clone, Deserialize, JsonSchema)]
pub struct TenthManToolArgs {
    /// The execution ID of the current agent run
    pub execution_id: String,

    /// Review mode (defaults to FullHistory)
    #[serde(default)]
    pub review_mode: ReviewMode,

    /// Type of review: "quick" (lightweight risk check) or "full" (comprehensive analysis)
    #[serde(default = "default_review_type")]
    pub review_type: String,

    /// Optional: specific focus area for the review
    #[serde(skip_serializing_if = "Option::is_none")]
    pub focus_area: Option<String>,
}

fn default_review_type() -> String {
    "full".to_string()
}

/// Tenth Man tool output
#[derive(Debug, Clone, Serialize)]
pub struct TenthManToolOutput {
    pub success: bool,
    pub critique: Option<String>,
    pub risk_level: String,
    pub message: String,
}

/// Tenth Man tool errors
#[derive(Debug, thiserror::Error)]
pub enum TenthManToolError {
    #[error(
        "LLM config not found for execution {0}. Make sure Tenth Man is properly initialized."
    )]
    ConfigNotFound(String),
    #[error("Review failed: {0}")]
    ReviewFailed(String),
    #[error("Internal error: {0}")]
    InternalError(String),
}

/// Tenth Man Tool implementation
#[derive(Debug, Clone, Default)]
pub struct TenthManTool;

impl TenthManTool {
    pub fn new() -> Self {
        Self
    }

    pub const NAME: &'static str = "tenth_man_review";
    pub const DESCRIPTION: &'static str = "Run a structured adversarial review (\"10th Man\") on the current execution before continuing. \
        \n\nUse this when you need a deliberate counterargument to challenge hidden assumptions, weak evidence, blind spots, risky tradeoffs, or repeated failed reasoning. \
        It is especially useful for ambiguous debugging, security reasoning, architecture choices, high-cost decisions, and any step that may be hard to undo.\
        \n\nDo not use this for simple fact lookup, deterministic low-risk execution, or formatting-only work where no real judgment call is involved.\
        \n\n[CRITICAL RULE]: Do not wait until you subjectively \"feel stuck\". You MUST call this tool before another retry when any of these are true:\
        \n- You have already tried 3-4 turns on the same path\
        \n- You are repeating the same tool family, route family, command pattern, or parameter pattern without clear new evidence\
        \n- Your next step is only a small variation of a failed attempt\
        \n- You cannot state what new information the next attempt is expected to produce\
        \n- Your current plan still depends on an assumption you have not verified\
        \n\nBefore repeating a path, ask yourself: \"What new evidence will this attempt produce?\" If the answer is weak, unclear, or mostly the same as before, call `tenth_man_review` first.\
        \n\nRecommended stuck-call:\
        \n- `review_mode`: `{ \"mode\": \"full_history\" }`\
        \n- `review_type`: `full`\
        \n\nThis review can surface:\
        \n- Hidden assumptions and weak evidence\
        \n- Logical gaps, contradictions, and blind spots\
        \n- Edge cases, constraints, and second-order effects\
        \n- Safer, simpler, or more robust alternative approaches\
        \n\nArguments:\
        \n- `execution_id` (required): Current execution ID\
        \n- `review_mode` (optional): `full_history` for the whole thread, or `recent_messages` with `count` for a narrow local review\
        \n- `review_type` (optional, default `full`): `quick` finds the highest-risk issue fast, `full` performs a deeper critique with tradeoffs and mitigations\
        \n- `focus_area` (optional): Short phrase describing what to stress test";
}

impl Tool for TenthManTool {
    const NAME: &'static str = Self::NAME;
    type Args = TenthManToolArgs;
    type Output = TenthManToolOutput;
    type Error = TenthManToolError;

    async fn definition(&self, _prompt: String) -> rig::completion::ToolDefinition {
        rig::completion::ToolDefinition {
            name: Self::NAME.to_string(),
            description: Self::DESCRIPTION.to_string(),
            parameters: serde_json::to_value(schemars::schema_for!(TenthManToolArgs))
                .unwrap_or_default(),
        }
    }

    async fn call(&self, args: Self::Args) -> Result<Self::Output, Self::Error> {
        tracing::info!(
            "Tenth Man review requested - execution_id: {}, review_type: {}, review_mode: {:?}, focus_area: {:?}",
            args.execution_id,
            args.review_type,
            args.review_mode,
            args.focus_area
        );

        // Get executor and call it
        let executor = get_executor()?;
        executor(args).await
    }
}
