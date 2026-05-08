//! Structured completion metadata for one agent turn.

use serde::Serialize;

use super::ToolCallRecord;

const TOOL_RESULT_EXCERPT_CHARS: usize = 500;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
pub enum AgentTurnStopReason {
    FinalAssistant,
    CancelledBeforeStart,
}

impl AgentTurnStopReason {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::FinalAssistant => "final_assistant",
            Self::CancelledBeforeStart => "cancelled_before_start",
        }
    }
}

#[derive(Debug, Clone, Copy, Default, Serialize)]
pub struct AgentToolProtocolSummary {
    pub tool_call_count: usize,
    pub tool_result_count: usize,
    pub pending_tool_call_count: usize,
    pub final_assistant_after_last_tool_result: bool,
}

#[derive(Debug, Clone, Serialize)]
pub struct AgentTurnToolSummary {
    pub id: String,
    pub name: String,
    pub success: bool,
    pub has_result: bool,
    pub result_excerpt: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
pub struct AgentTurnOutcome {
    pub final_response: String,
    pub stop_reason: AgentTurnStopReason,
    pub tool_protocol: AgentToolProtocolSummary,
    pub recent_tools: Vec<AgentTurnToolSummary>,
}

impl AgentTurnOutcome {
    pub fn direct(final_response: String) -> Self {
        Self {
            final_response,
            stop_reason: AgentTurnStopReason::FinalAssistant,
            recent_tools: Vec::new(),
            tool_protocol: AgentToolProtocolSummary {
                final_assistant_after_last_tool_result: true,
                ..AgentToolProtocolSummary::default()
            },
        }
    }

    pub fn cancelled_before_start() -> Self {
        Self {
            final_response: String::new(),
            stop_reason: AgentTurnStopReason::CancelledBeforeStart,
            tool_protocol: AgentToolProtocolSummary::default(),
            recent_tools: Vec::new(),
        }
    }

    pub fn tool_run(
        final_response: String,
        tool_calls: &[ToolCallRecord],
        pending_tool_call_count: usize,
        final_assistant_after_last_tool_result: bool,
    ) -> Self {
        let tool_call_count = tool_calls.len();
        let tool_result_count = tool_calls
            .iter()
            .filter(|call| call.result.is_some())
            .count();
        let missing_results = tool_call_count.saturating_sub(tool_result_count);
        let pending_tool_call_count = pending_tool_call_count.max(missing_results);

        Self {
            tool_protocol: AgentToolProtocolSummary {
                tool_call_count,
                tool_result_count,
                pending_tool_call_count,
                final_assistant_after_last_tool_result: tool_call_count == 0
                    || final_assistant_after_last_tool_result,
            },
            recent_tools: recent_tool_summaries(tool_calls),
            final_response,
            stop_reason: AgentTurnStopReason::FinalAssistant,
        }
    }
}

fn recent_tool_summaries(tool_calls: &[ToolCallRecord]) -> Vec<AgentTurnToolSummary> {
    let mut summaries = tool_calls
        .iter()
        .rev()
        .take(5)
        .map(|call| AgentTurnToolSummary {
            id: call.id.clone(),
            name: call.name.clone(),
            success: call.success,
            has_result: call.result.is_some(),
            result_excerpt: call
                .result
                .as_deref()
                .map(|result| result.chars().take(TOOL_RESULT_EXCERPT_CHARS).collect()),
        })
        .collect::<Vec<_>>();
    summaries.reverse();
    summaries
}
