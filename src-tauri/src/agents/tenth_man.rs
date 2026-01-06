//! The Tenth Man Rule - Adversarial Review Logic
//! 
//! "If nine of us with the same information arrived at the exact same conclusion, 
//! it’s the duty of the tenth man to disagree. No matter how unlikely it may seem."
//! -- World War Z

use anyhow::Result;
use crate::agents::executor::AgentExecuteParams;
use sentinel_llm::{LlmClient, LlmConfig};

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

pub struct TenthMan {
    config: LlmConfig,
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

        Self {
            config: llm_config,
        }
    }

    /// Reviews the current conversation history and the proposed conclusion/plan.
    /// Returns the critique string.
    pub async fn review(
        &self, 
        task: &str,
        context_summary: &str, 
        last_assistant_message: &str
    ) -> Result<String> {
        let client = LlmClient::new(self.config.clone());
        
        let prompt = format!(
            "### Original Task:\n{}\n\n### Context Summary:\n{}\n\n### Proposed Conclusion/Plan:\n{}\n\n---\n\nPerform your Tenth Man review now. Attack this conclusion.",
            task,
            context_summary,
            last_assistant_message
        );

        let critique = client.completion(
            Some(TENTH_MAN_SYSTEM_PROMPT),
            &prompt
        ).await?;

        Ok(critique)
    }
}
