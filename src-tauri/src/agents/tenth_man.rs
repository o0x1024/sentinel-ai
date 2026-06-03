//! The Tenth Man Rule - Adversarial Review Logic
//!
//! "If nine of us with the same information arrived at the exact same conclusion,
//! it's the duty of the tenth man to disagree. No matter how unlikely it may seem."
//! -- World War Z

use crate::agents::executor::AgentExecuteParams;
use anyhow::Result;
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
    pub recent_failure_count: usize,
    pub last_tool_name: Option<String>,
    pub has_recent_verification: bool,
    pub has_side_effects: bool,
    pub current_content: Option<String>,
    pub trigger_reason: TriggerReason,
}

/// Reason for triggering intervention
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum TriggerReason {
    ToolCallThreshold,
    HighRiskTool(String),
    RepeatedFailurePattern,
    LoopDetected,
    LowEvidenceHighConfidence,
    DangerousKeyword(String),
    ConclusionDetected,
    FinalResponse,
    FinalResponseWithoutVerification,
    Manual,
}

/// Configuration for Tenth Man
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TenthManConfig {
    pub mode: InterventionMode,
    pub auto_inject_to_context: bool,
    pub require_user_confirmation: bool,
    #[serde(default)]
    pub trigger_policy: TenthManTriggerPolicy,
}

impl Default for TenthManConfig {
    fn default() -> Self {
        Self {
            mode: InterventionMode::default(),
            auto_inject_to_context: false,
            require_user_confirmation: false,
            trigger_policy: TenthManTriggerPolicy::default(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TenthManTriggerPolicy {
    #[serde(default = "default_true")]
    pub review_high_risk_tools: bool,
    #[serde(default = "default_true")]
    pub review_repeated_failures: bool,
    #[serde(default = "default_true")]
    pub review_loops: bool,
    #[serde(default = "default_true")]
    pub review_final_response_without_verification: bool,
    #[serde(default = "default_true")]
    pub review_low_evidence_high_confidence: bool,
    #[serde(default = "default_repeated_failure_streak")]
    pub repeated_failure_streak: usize,
    #[serde(default = "default_loop_repeat_threshold")]
    pub loop_repeat_threshold: usize,
    #[serde(default = "default_recent_verification_window")]
    pub recent_verification_window: usize,
    #[serde(default = "default_minimum_evidence_tool_calls")]
    pub minimum_evidence_tool_calls: usize,
    #[serde(default = "default_minimum_evidence_score")]
    pub minimum_evidence_score: u32,
}

impl Default for TenthManTriggerPolicy {
    fn default() -> Self {
        Self {
            review_high_risk_tools: true,
            review_repeated_failures: true,
            review_loops: true,
            review_final_response_without_verification: true,
            review_low_evidence_high_confidence: true,
            repeated_failure_streak: default_repeated_failure_streak(),
            loop_repeat_threshold: default_loop_repeat_threshold(),
            recent_verification_window: default_recent_verification_window(),
            minimum_evidence_tool_calls: default_minimum_evidence_tool_calls(),
            minimum_evidence_score: default_minimum_evidence_score(),
        }
    }
}

impl TenthManTriggerPolicy {
    pub fn repeated_failure_streak(&self) -> usize {
        self.repeated_failure_streak.max(1)
    }

    pub fn loop_repeat_threshold(&self) -> usize {
        self.loop_repeat_threshold.max(1)
    }

    pub fn recent_verification_window(&self) -> usize {
        self.recent_verification_window.max(1)
    }

    pub fn minimum_evidence_tool_calls(&self) -> usize {
        self.minimum_evidence_tool_calls.max(1)
    }

    pub fn minimum_evidence_score(&self) -> u32 {
        self.minimum_evidence_score
            .max(self.minimum_evidence_tool_calls() as u32)
            .max(1)
    }
}

const fn default_true() -> bool {
    true
}

const fn default_repeated_failure_streak() -> usize {
    2
}

const fn default_loop_repeat_threshold() -> usize {
    2
}

const fn default_recent_verification_window() -> usize {
    3
}

const fn default_minimum_evidence_tool_calls() -> usize {
    2
}

const fn default_minimum_evidence_score() -> u32 {
    3
}

pub struct TenthMan {
    config: LlmConfig,
    intervention_mode: InterventionMode,
}

impl TenthMan {
    pub fn new(params: &AgentExecuteParams) -> Self {
        let rig_provider = params.rig_provider.to_lowercase();
        let mut llm_config =
            LlmConfig::new(&rig_provider, &params.model).with_rig_provider(&rig_provider);
        if params.timeout_secs == 0 {
            llm_config = llm_config.without_timeout();
        } else {
            llm_config = llm_config.with_timeout(params.timeout_secs);
        }

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
            InterventionMode::SystemOnly => self.should_auto_trigger(context, true),
            InterventionMode::Hybrid {
                force_final_review, ..
            } => self.should_auto_trigger(context, *force_final_review),
            InterventionMode::FinalOnly => false,
            InterventionMode::Proactive {
                tool_call_interval,
                dangerous_keywords,
            } => {
                if self.should_auto_trigger(context, true) {
                    return true;
                }

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

    fn should_auto_trigger(&self, context: &InterventionContext, force_final_review: bool) -> bool {
        match &context.trigger_reason {
            TriggerReason::HighRiskTool(_)
            | TriggerReason::RepeatedFailurePattern
            | TriggerReason::LoopDetected
            | TriggerReason::LowEvidenceHighConfidence
            | TriggerReason::FinalResponseWithoutVerification
            | TriggerReason::Manual => true,
            TriggerReason::FinalResponse => force_final_review,
            _ => false,
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
            "因此",
            "所以",
            "综上所述",
            "总结",
            "结论",
            "我建议",
            "应该",
            "必须",
            "最佳方案",
            "therefore",
            "in conclusion",
            "to summarize",
            "I recommend",
            "should",
            "must",
            "best approach",
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
            "Task: {}\nExecution ID: {}\nTrigger Reason: {}\nTool Calls So Far: {}\nRecent Failure Count: {}\nLast Tool: {}\nHas Side Effects: {}\nHas Recent Verification: {}\n\nCurrent Content:\n{}\n\n---\n\nQuick risk assessment:",
            context.task,
            context.execution_id,
            Self::describe_trigger_reason(&context.trigger_reason),
            context.tool_call_count,
            context.recent_failure_count,
            context
                .last_tool_name
                .as_deref()
                .unwrap_or("none"),
            if context.has_side_effects { "yes" } else { "no" },
            if context.has_recent_verification { "yes" } else { "no" },
            content
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

    fn describe_trigger_reason(trigger_reason: &TriggerReason) -> String {
        match trigger_reason {
            TriggerReason::ToolCallThreshold => "tool_call_threshold".to_string(),
            TriggerReason::HighRiskTool(tool) => format!("high_risk_tool:{}", tool),
            TriggerReason::RepeatedFailurePattern => "repeated_failure_pattern".to_string(),
            TriggerReason::LoopDetected => "loop_detected".to_string(),
            TriggerReason::LowEvidenceHighConfidence => "low_evidence_high_confidence".to_string(),
            TriggerReason::DangerousKeyword(keyword) => {
                format!("dangerous_keyword:{}", keyword)
            }
            TriggerReason::ConclusionDetected => "conclusion_detected".to_string(),
            TriggerReason::FinalResponse => "final_response".to_string(),
            TriggerReason::FinalResponseWithoutVerification => {
                "final_response_without_verification".to_string()
            }
            TriggerReason::Manual => "manual".to_string(),
        }
    }

    /// Full review (comprehensive analysis) - Legacy method
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

    /// Review with complete history (System mode uses this)
    pub async fn review_with_history(&self, execution_id: &str) -> Result<String> {
        use crate::agents::tenth_man_executor::execute_tenth_man_review;
        use sentinel_tools::buildin_tools::tenth_man_tool::{ReviewMode, TenthManToolArgs};

        let args = TenthManToolArgs {
            execution_id: execution_id.to_string(),
            review_mode: ReviewMode::FullHistory,
            review_type: "full".to_string(),
            focus_area: Some("final solution and complete execution process".to_string()),
        };

        let output = execute_tenth_man_review(args)
            .await
            .map_err(|e| anyhow::anyhow!("Review failed: {}", e))?;

        output
            .critique
            .ok_or_else(|| anyhow::anyhow!("No critique generated"))
    }
}
