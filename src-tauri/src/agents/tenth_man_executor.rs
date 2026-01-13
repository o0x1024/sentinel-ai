//! Tenth Man Executor - Runtime logic for Tenth Man tool
//!
//! This module contains the actual LLM execution logic for the Tenth Man tool.
//! It's separated from the tool definition to avoid dependency issues.

use sentinel_llm::{LlmClient, LlmConfig};
use sentinel_tools::buildin_tools::tenth_man_tool::{
    TenthManToolArgs, TenthManToolOutput, TenthManToolError
};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;
use once_cell::sync::Lazy;

/// Global LLM config storage for Tenth Man reviews (set per execution)
static TENTH_MAN_CONFIGS: Lazy<Arc<RwLock<HashMap<String, LlmConfig>>>> = 
    Lazy::new(|| Arc::new(RwLock::new(HashMap::new())));

/// Global task context storage (for providing context to reviews)
static TASK_CONTEXTS: Lazy<Arc<RwLock<HashMap<String, String>>>> = 
    Lazy::new(|| Arc::new(RwLock::new(HashMap::new())));

/// System prompt for quick review
const TENTH_MAN_QUICK_REVIEW_PROMPT: &str = r#"You are the "Tenth Man" performing a rapid risk assessment.

Quickly scan the content and identify ONLY the most severe risk (if any).
If there's no significant risk, respond with "无严重风险".
If there IS a risk, provide a 1-2 sentence warning.

Be extremely concise. Focus on HIGH-IMPACT risks only.

**IMPORTANT**: You must answer in Chinese (Simplified Chinese).
"#;

/// System prompt for full review
const TENTH_MAN_FULL_REVIEW_PROMPT: &str = r#"You are the "Tenth Man".
Your role is to act as a fail-safe mechanism against Groupthink and confirmation bias.

The agent has analyzed a situation and reached a conclusion or plan.
Your absolute DUTY is to challenge this conclusion. You must assume the conclusion is WRONG, DANGEROUS, or INCOMPLETE.

### Your Objectives:
1. **Identify False Assumptions**: What underlying premises are taken for granted but might be false?
2. **Find the "Black Swan"**: What low-probability but high-impact scenario has been ignored?
3. **Attack the Logic**: Where are the leaps in reasoning?
4. **Security Audit**: If this is a security operation, how would a sophisticated attacker bypass this plan?

### Response Format:
You must be direct, critical, and concise. Do not be polite.
If you find no significant flaws, you must still present the "Least Likely but Most Dangerous" failure mode.

Structure your response as:
**[Tenth Man Intervention]**
**1. Critical Flaw**: (The biggest weakness)
**2. Hidden Risk**: (The overlooked scenario)
**3. Counter-Argument**: (Why the current plan might fail)

**IMPORTANT**: You must answer in Chinese (Simplified Chinese).
"#;

/// Set LLM config for a specific execution
pub async fn set_tenth_man_config(execution_id: String, config: LlmConfig) {
    let mut configs = TENTH_MAN_CONFIGS.write().await;
    configs.insert(execution_id, config);
}

/// Set task context for a specific execution
pub async fn set_task_context(execution_id: String, task: String) {
    let mut contexts = TASK_CONTEXTS.write().await;
    contexts.insert(execution_id, task);
}

/// Clear config and context for an execution (cleanup)
pub async fn clear_tenth_man_execution(execution_id: &str) {
    let mut configs = TENTH_MAN_CONFIGS.write().await;
    configs.remove(execution_id);
    let mut contexts = TASK_CONTEXTS.write().await;
    contexts.remove(execution_id);
}

/// Assess risk level from critique content
fn assess_risk_level(critique: &str) -> String {
    let critique_lower = critique.to_lowercase();
    
    if critique.contains("无严重风险") || critique.contains("no significant risk") {
        return "none".to_string();
    }
    
    // Critical risk indicators
    if critique_lower.contains("critical") 
        || critique_lower.contains("严重")
        || critique_lower.contains("致命")
        || critique_lower.contains("危险")
        || critique_lower.contains("disaster") {
        return "critical".to_string();
    }
    
    // High risk indicators
    if critique_lower.contains("high risk")
        || critique_lower.contains("高风险")
        || critique_lower.contains("重大缺陷")
        || critique_lower.contains("major flaw") {
        return "high".to_string();
    }
    
    // Medium risk indicators
    if critique_lower.contains("medium")
        || critique_lower.contains("中等")
        || critique_lower.contains("potential issue") {
        return "medium".to_string();
    }
    
    // Default to low risk if critique exists
    "low".to_string()
}

/// Execute Tenth Man review
pub async fn execute_tenth_man_review(args: TenthManToolArgs) -> Result<TenthManToolOutput, TenthManToolError> {
    tracing::info!(
        "Executing Tenth Man review - execution_id: {}, review_type: {}",
        args.execution_id,
        args.review_type
    );
    
    // Get LLM config for this execution
    let config = {
        let configs = TENTH_MAN_CONFIGS.read().await;
        configs.get(&args.execution_id).cloned()
    };
    
    let Some(config) = config else {
        return Err(TenthManToolError::ConfigNotFound(args.execution_id.clone()));
    };
    
    // Get task context
    let task_context = {
        let contexts = TASK_CONTEXTS.read().await;
        contexts.get(&args.execution_id).cloned().unwrap_or_else(|| "Unknown task".to_string())
    };
    
    // Build review prompt
    let context_desc = args.context_description
        .as_deref()
        .unwrap_or("Agent's current work");
    
    let review_prompt = match args.review_type.as_str() {
        "quick" => {
            format!(
                "### Original Task:\n{}\n\n### Current Context:\n{}\n\n### Content to Review:\n{}\n\n---\n\nPerform quick risk assessment:",
                task_context, context_desc, args.content_to_review
            )
        }
        "full" | _ => {
            format!(
                "### Original Task:\n{}\n\n### Current Context:\n{}\n\n### Content to Review:\n{}\n\n---\n\nPerform your Tenth Man review now. Attack this conclusion.",
                task_context, context_desc, args.content_to_review
            )
        }
    };
    
    let system_prompt = match args.review_type.as_str() {
        "quick" => TENTH_MAN_QUICK_REVIEW_PROMPT,
        "full" | _ => TENTH_MAN_FULL_REVIEW_PROMPT,
    };
    
    // Perform review
    let client = LlmClient::new(config);
    let critique = client
        .completion(Some(system_prompt), &review_prompt)
        .await
        .map_err(|e| TenthManToolError::ReviewFailed(e.to_string()))?;
    
    // Assess risk level
    let risk_level = assess_risk_level(&critique);
    
    // Check if no risk found
    let success = !critique.trim().is_empty();
    let message = if risk_level == "none" {
        "No significant risks identified".to_string()
    } else {
        format!("Review completed - Risk level: {}", risk_level)
    };
    
    tracing::info!(
        "Tenth Man review completed - execution_id: {}, risk_level: {}, critique_length: {}",
        args.execution_id,
        risk_level,
        critique.len()
    );
    
    Ok(TenthManToolOutput {
        success,
        critique: Some(critique),
        risk_level,
        message,
    })
}

/// Initialize Tenth Man executor
pub fn init_tenth_man_executor() {
    use sentinel_tools::buildin_tools::tenth_man_tool::set_tenth_man_executor;
    
    let executor = std::sync::Arc::new(|args: TenthManToolArgs| {
        Box::pin(execute_tenth_man_review(args)) as std::pin::Pin<Box<dyn std::future::Future<Output = _> + Send>>
    });
    
    set_tenth_man_executor(executor);
    tracing::info!("Tenth Man executor initialized");
}
