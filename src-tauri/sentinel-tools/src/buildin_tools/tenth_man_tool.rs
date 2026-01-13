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


/// Tenth Man tool arguments
#[derive(Debug, Clone, Deserialize, JsonSchema)]
pub struct TenthManToolArgs {
    /// The execution ID of the current agent run
    pub execution_id: String,
    /// Content to review (e.g., current plan, proposed solution, or conclusion)
    pub content_to_review: String,
    /// Context description (what this review is about)
    pub context_description: Option<String>,
    /// Type of review: "quick" (lightweight risk check) or "full" (comprehensive analysis)
    #[serde(default = "default_review_type")]
    pub review_type: String,
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
    pub const DESCRIPTION: &'static str = "Request an adversarial review of your current plan or conclusion. The Tenth Man will challenge your assumptions, identify hidden risks, and find potential flaws. Use 'quick' review for rapid risk checks, or 'full' review for comprehensive analysis. This tool helps prevent groupthink and confirmation bias.";
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
            "Tenth Man review requested - execution_id: {}, review_type: {}, context: {:?}",
            args.execution_id,
            args.review_type,
            args.context_description
        );
        
        // Get executor and call it
        let executor = get_executor()?;
        executor(args).await
    }
}
