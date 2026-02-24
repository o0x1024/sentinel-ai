//! Context policies and scopes.

#[derive(Debug, Clone)]
pub struct ContextBudgetPolicy {
    pub system_max_tokens: usize,
    pub run_state_max_tokens: usize,
    pub window_max_tokens: usize,
    pub retrieval_max_tokens: usize,
    pub tool_digest_max_tokens: usize,
}

impl ContextBudgetPolicy {
    /// Scale budget proportionally based on the actual model context window.
    /// The default budget is calibrated for 128K; this scales linearly with a
    /// reasonable per-section cap so tiny models aren't over-allocated and huge
    /// models (Gemini 1M) don't get absurdly large budgets.
    pub fn scale_to_context(&self, max_context_tokens: usize) -> Self {
        if max_context_tokens == 0 {
            return self.clone();
        }
        let ratio = (max_context_tokens as f64 / 128_000.0).max(1.0);
        let safe = (max_context_tokens as f64 * 0.85) as usize;
        Self {
            system_max_tokens: scale_value(self.system_max_tokens, ratio, safe / 4),
            run_state_max_tokens: scale_value(self.run_state_max_tokens, ratio, safe / 8),
            window_max_tokens: scale_value(self.window_max_tokens, ratio, (safe as f64 * 0.55) as usize),
            retrieval_max_tokens: scale_value(self.retrieval_max_tokens, ratio, safe / 8),
            tool_digest_max_tokens: scale_value(self.tool_digest_max_tokens, ratio, safe / 10),
        }
    }
}

fn scale_value(base: usize, ratio: f64, cap: usize) -> usize {
    ((base as f64 * ratio) as usize).min(cap)
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ContextScope {
    Agent,
    Subagent,
}

#[derive(Debug, Clone)]
pub struct ContextPolicy {
    pub scope: ContextScope,
    pub include_working_dir: bool,
    pub include_context_storage: bool,
    pub include_task_mainline: bool,
    pub include_run_state: bool,
    pub include_document_attachments: bool,
    pub include_skill_instructions: bool,
    pub run_state_max_digests: usize,
    pub run_state_max_chars: usize,
    pub task_brief_max_chars: usize,
    pub layer_max_chars: usize,
    pub feature_context_packet_v2: bool,
    pub budget: ContextBudgetPolicy,
}

impl Default for ContextPolicy {
    fn default() -> Self {
        Self {
            scope: ContextScope::Agent,
            include_working_dir: true,
            include_context_storage: true,
            include_task_mainline: true,
            include_run_state: true,
            include_document_attachments: true,
            include_skill_instructions: true,
            run_state_max_digests: 6,
            run_state_max_chars: 2400,
            task_brief_max_chars: 600,
            layer_max_chars: 12000,
            feature_context_packet_v2: true,
            budget: ContextBudgetPolicy {
                system_max_tokens: 4000,
                run_state_max_tokens: 1800,
                window_max_tokens: 12000,
                retrieval_max_tokens: 2400,
                tool_digest_max_tokens: 1800,
            },
        }
    }
}

impl ContextPolicy {
    pub fn subagent() -> Self {
        Self {
            scope: ContextScope::Subagent,
            include_working_dir: false,
            include_context_storage: false,
            include_task_mainline: false,
            include_run_state: true,
            include_document_attachments: false,
            include_skill_instructions: false,
            run_state_max_digests: 4,
            run_state_max_chars: 1600,
            task_brief_max_chars: 400,
            layer_max_chars: 8000,
            feature_context_packet_v2: true,
            budget: ContextBudgetPolicy {
                system_max_tokens: 2500,
                run_state_max_tokens: 1200,
                window_max_tokens: 7000,
                retrieval_max_tokens: 1200,
                tool_digest_max_tokens: 1000,
            },
        }
    }

    /// Subagent in audit mode: keep subagent scope isolation, but provide
    /// larger budgets and more run-state context for evidence-heavy analysis.
    pub fn subagent_audit() -> Self {
        Self {
            scope: ContextScope::Subagent,
            include_working_dir: false,
            include_context_storage: false,
            include_task_mainline: false,
            include_run_state: true,
            include_document_attachments: false,
            include_skill_instructions: false,
            run_state_max_digests: 8,
            run_state_max_chars: 3200,
            task_brief_max_chars: 600,
            layer_max_chars: 14000,
            feature_context_packet_v2: true,
            budget: ContextBudgetPolicy {
                system_max_tokens: 4200,
                run_state_max_tokens: 2200,
                window_max_tokens: 12000,
                retrieval_max_tokens: 2200,
                tool_digest_max_tokens: 1800,
            },
        }
    }

    /// Audit mode: larger budgets to accommodate long multi-phase code analysis.
    /// - More tool digests (many audit tools active simultaneously)
    /// - Larger run state (tracks phase progress, coverage, found patterns)
    /// - Larger context window (long code snippets and file content)
    /// - Larger system budget (three-phase audit instruction is verbose)
    pub fn audit() -> Self {
        Self {
            scope: ContextScope::Agent,
            include_working_dir: true,
            include_context_storage: true,
            include_task_mainline: true,
            include_run_state: true,
            include_document_attachments: true,
            include_skill_instructions: true,
            run_state_max_digests: 12,
            run_state_max_chars: 4800,
            task_brief_max_chars: 800,
            layer_max_chars: 20000,
            feature_context_packet_v2: true,
            budget: ContextBudgetPolicy {
                system_max_tokens: 7000,
                run_state_max_tokens: 3600,
                window_max_tokens: 20000,
                retrieval_max_tokens: 3600,
                tool_digest_max_tokens: 3200,
            },
        }
    }
}
