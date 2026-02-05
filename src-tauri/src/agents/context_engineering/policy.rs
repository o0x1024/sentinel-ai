//! Context policies and scopes.

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
        }
    }
}
