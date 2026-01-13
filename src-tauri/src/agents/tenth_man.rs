//! The Tenth Man Rule - Adversarial Review Logic
//! 
//! "If nine of us with the same information arrived at the exact same conclusion, 
//! it's the duty of the tenth man to disagree. No matter how unlikely it may seem."
//! -- World War Z

use anyhow::Result;
use crate::agents::executor::AgentExecuteParams;
use sentinel_llm::{LlmClient, LlmConfig};
use serde::{Deserialize, Serialize};

/// System prompt for the Tenth Man (The Devil's Advocate)
const TENTH_MAN_SYSTEM_PROMPT: &str = r#"You are the "Tenth Man".
Your role is to act as a fail-safe mechanism against Groupthink and confirmation bias.

The system or agent has analyzed a situation and reached a conclusion or plan.
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

/// Quick review prompt for lightweight checks
const TENTH_MAN_QUICK_REVIEW_PROMPT: &str = r#"You are the "Tenth Man" performing a rapid risk assessment.

Quickly scan the content and identify ONLY the most severe risk (if any).
If there's no significant risk, respond with "无严重风险".
If there IS a risk, provide a 1-2 sentence warning.

Be extremely concise. Focus on HIGH-IMPACT risks only.

**IMPORTANT**: You must answer in Chinese (Simplified Chinese).
"#;

/// Intervention mode for the Tenth Man
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum InterventionMode {
    /// Tool-only: Only available as LLM-callable tool (no automatic reviews)
    ToolOnly,
    /// System-only: Only system automatic reviews (LLM cannot call)
    SystemOnly,
    /// Hybrid: LLM can call as tool + System enforces final review
    Hybrid {
        /// LLM can call the tool
        tool_available: bool,
        /// System forces final review
        force_final_review: bool,
        /// Track if LLM has called recently
        #[serde(skip)]
        last_tool_call_time: Option<std::time::Instant>,
    },
    /// Legacy modes (deprecated but kept for compatibility)
    #[serde(rename = "final_only")]
    FinalOnly,
    #[serde(rename = "proactive")]
    Proactive {
        /// Trigger after every N tool calls
        tool_call_interval: Option<usize>,
        /// Trigger when dangerous keywords detected
        dangerous_keywords: Vec<String>,
    },
    #[serde(rename = "realtime")]
    Realtime,
}

impl Default for InterventionMode {
    fn default() -> Self {
        InterventionMode::Hybrid {
            tool_available: true,
            force_final_review: true,
            last_tool_call_time: None,
        }
    }
}

/// Trigger conditions for intervention
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TriggerConditions {
    /// Review before tool execution
    pub before_tool_execution: bool,
    /// Review when conclusion detected
    pub on_conclusion_detected: bool,
    /// Review on final response
    pub on_final_response: bool,
}

impl Default for TriggerConditions {
    fn default() -> Self {
        Self {
            before_tool_execution: false,
            on_conclusion_detected: true,
            on_final_response: true,
        }
    }
}

/// Context for intervention decision
#[derive(Debug, Clone)]
pub struct InterventionContext {
    pub execution_id: String,
    pub task: String,
    pub tool_call_count: usize,
    pub current_content: Option<String>,
    pub trigger_reason: TriggerReason,
}

/// Reason for triggering intervention
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum TriggerReason {
    ToolCallThreshold,
    DangerousKeyword(String),
    ConclusionDetected,
    FinalResponse,
    Manual,
}

/// Configuration for Tenth Man
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TenthManConfig {
    pub mode: InterventionMode,
    pub auto_inject_to_context: bool,
    pub require_user_confirmation: bool,
}

impl Default for TenthManConfig {
    fn default() -> Self {
        Self {
            mode: InterventionMode::default(),
            auto_inject_to_context: false,
            require_user_confirmation: false,
        }
    }
}


pub struct TenthMan {
    config: LlmConfig,
    intervention_mode: InterventionMode,
}

impl TenthMan {
    pub fn new(params: &AgentExecuteParams) -> Self {
        let rig_provider = params.rig_provider.to_lowercase();
        let mut llm_config = LlmConfig::new(&rig_provider, &params.model)
            .with_timeout(params.timeout_secs)
            .with_rig_provider(&rig_provider);

        if let Some(ref api_key) = params.api_key {
            llm_config = llm_config.with_api_key(api_key);
        }
        if let Some(ref api_base) = params.api_base {
            llm_config = llm_config.with_base_url(api_base);
        }

        let intervention_mode = params
            .tenth_man_config
            .as_ref()
            .map(|c| c.mode.clone())
            .unwrap_or_default();

        Self {
            config: llm_config,
            intervention_mode,
        }
    }

    /// Check if intervention should be triggered
    pub fn should_trigger(&self, context: &InterventionContext) -> bool {
        match &self.intervention_mode {
            InterventionMode::ToolOnly => false, // Never auto-trigger, only via tool calls
            InterventionMode::SystemOnly => {
                // Always trigger on final response
                matches!(context.trigger_reason, TriggerReason::FinalResponse)
            }
            InterventionMode::Hybrid { force_final_review, .. } => {
                // Trigger on final response if forced
                if *force_final_review && matches!(context.trigger_reason, TriggerReason::FinalResponse) {
                    return true;
                }
                false
            }
            InterventionMode::FinalOnly => false,
            InterventionMode::Proactive {
                tool_call_interval,
                dangerous_keywords,
            } => {
                // Check tool call count
                if let Some(interval) = tool_call_interval {
                    if context.tool_call_count > 0 && context.tool_call_count % interval == 0 {
                        return true;
                    }
                }

                // Check dangerous keywords
                if !dangerous_keywords.is_empty() {
                    if let Some(ref content) = context.current_content {
                        for keyword in dangerous_keywords {
                            if content.contains(keyword) {
                                return true;
                            }
                        }
                    }
                }

                // Check conclusion markers
                if let Some(ref content) = context.current_content {
                    if Self::contains_conclusion_markers(content) {
                        return true;
                    }
                }

                false
            }
            InterventionMode::Realtime => true,
        }
    }
    
    /// Check if tool is available for LLM to call
    pub fn is_tool_available(&self) -> bool {
        match &self.intervention_mode {
            InterventionMode::ToolOnly => true,
            InterventionMode::Hybrid { tool_available, .. } => *tool_available,
            _ => false,
        }
    }

    /// Detect conclusion markers in content
    pub fn contains_conclusion_markers(content: &str) -> bool {
        let markers = [
            "因此", "所以", "综上所述", "总结", "结论",
            "我建议", "应该", "必须", "最佳方案",
            "therefore", "in conclusion", "to summarize",
            "I recommend", "should", "must", "best approach",
        ];
        markers.iter().any(|m| content.contains(m))
    }

    /// Quick review for real-time monitoring (lightweight)
    pub async fn quick_review(&self, context: &InterventionContext) -> Result<Option<String>> {
        let content = match &context.current_content {
            Some(c) => c,
            None => return Ok(None),
        };

        let prompt = format!(
            "Task: {}\n\nCurrent Content:\n{}\n\n---\n\nQuick risk assessment:",
            context.task, content
        );

        let client = LlmClient::new(self.config.clone());
        let critique = client
            .completion(Some(TENTH_MAN_QUICK_REVIEW_PROMPT), &prompt)
            .await?;

        // If no significant risk, return None
        if critique.contains("无严重风险") || critique.contains("no significant risk") {
            Ok(None)
        } else {
            Ok(Some(critique))
        }
    }

    /// Full review (comprehensive analysis)
    pub async fn review(
        &self,
        task: &str,
        context_summary: &str,
        last_assistant_message: &str,
    ) -> Result<String> {
        let client = LlmClient::new(self.config.clone());

        let prompt = format!(
            "### Original Task:\n{}\n\n### Context Summary:\n{}\n\n### Proposed Conclusion/Plan:\n{}\n\n---\n\nPerform your Tenth Man review now. Attack this conclusion.",
            task, context_summary, last_assistant_message
        );

        let critique = client
            .completion(Some(TENTH_MAN_SYSTEM_PROMPT), &prompt)
            .await?;

        Ok(critique)
    }
}
