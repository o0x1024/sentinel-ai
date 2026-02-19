//! Tenth Man Tool - LLM-callable adversarial review tool
//!
//! Allows the LLM to proactively request critical review of its plans and conclusions.
//! Note: This tool is just the definition. The actual execution logic is in src-tauri/src/agents/tenth_man_executor.rs

use rig::tool::Tool;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

/// Type alias for the actual executor function (provided by main crate)
pub type TenthManExecutorFn = std::sync::Arc<
    dyn Fn(TenthManToolArgs) 
        -> std::pin::Pin<Box<dyn std::future::Future<Output = Result<TenthManToolOutput, TenthManToolError>> + Send>> 
        + Send 
        + Sync
>;

/// Global executor storage (set by main crate at runtime)
static TENTH_MAN_EXECUTOR: once_cell::sync::OnceCell<TenthManExecutorFn> = once_cell::sync::OnceCell::new();

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
pub enum ReviewMode {
    /// Review complete history (using sliding window summarization)
    FullHistory,
    /// Review recent N messages
    RecentMessages { 
        #[schemars(description = "Number of recent messages to review")]
        count: usize 
    },
    /// Review specific content (backward compatible)
    SpecificContent { 
        #[schemars(description = "Specific content to review")]
        content: String 
    },
}

impl Default for ReviewMode {
    fn default() -> Self {
        ReviewMode::FullHistory
    }
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
    "quick".to_string()
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
    #[error("LLM config not found for execution {0}. Make sure Tenth Man is properly initialized.")]
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
    pub const DESCRIPTION: &'static str = "Run a structured adversarial review (\"10th Man\") on the current work. \
        \n\nThis tool challenges prevailing assumptions and stress-tests the plan, analysis, or conclusion against alternatives, uncertainty, and failure modes.\
        \n\nIt can review complete conversation history (not just the latest message) to surface:\
        \n- Hidden assumptions and weak evidence\
        \n- Logical gaps, contradictions, and blind spots\
        \n- Edge cases, constraints, and second-order effects\
        \n- Safer or more robust alternative approaches\
        \n\nReview modes:\
        \n- 'full_history' (default): Review the full thread with summarization\
        \n- 'recent_messages': Review only the last N messages (set count)\
        \n\nReview types:\
        \n- 'quick': Short risk-oriented challenge\
        \n- 'full': Detailed critique with tradeoffs and mitigation ideas\
        \n\nUseful for any domain where quality of reasoning matters: decision-making, planning, coding, security, product strategy, operations, policy, and communications.";
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
