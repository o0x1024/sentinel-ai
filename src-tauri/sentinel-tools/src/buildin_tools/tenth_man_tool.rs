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
    pub const DESCRIPTION: &'static str = "Request adversarial review of your work from the Tenth Man. \
        \n\nThe Tenth Man reviews your COMPLETE conversation history (not just current message) to find:\
        \n- Logic flaws across multiple steps\
        \n- Dangerous assumptions in your reasoning\
        \n- Overlooked risks and edge cases\
        \n- Inconsistencies in your approach\
        \n\nReview modes:\
        \n- 'full_history' (default): Reviews entire conversation with smart summarization\
        \n- 'recent_messages': Reviews last N messages only (specify count)\
        \n- 'specific_content': Reviews a specific piece of content\
        \n\nReview types:\
        \n- 'quick': Fast risk identification (1-2 sentences)\
        \n- 'full': Comprehensive analysis with detailed critique\
        \n\nUse this tool when:\
        \n- Before executing critical operations\
        \n- After making important decisions\
        \n- When you want to validate your approach\
        \n- To catch mistakes before they cause problems";
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
