//! Agent turn completion gate aligned with tool-call protocol state.

use crate::agents::{AgentToolProtocolSummary, AgentTurnOutcome};

use super::ai_runtime_harness::{AgentHarnessMode, AgentHarnessSuccessAssessment};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AgentTurnIncompleteReason {
    PendingToolCalls,
    MissingFinalAssistantAfterToolResult,
    EmptyFinalAssistant,
}

impl AgentTurnIncompleteReason {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::PendingToolCalls => "tool_calls_pending",
            Self::MissingFinalAssistantAfterToolResult => {
                "stopped_after_tool_result_without_final_answer"
            }
            Self::EmptyFinalAssistant => "empty_final_assistant",
        }
    }

    fn message(self) -> &'static str {
        match self {
            Self::PendingToolCalls => "Agent turn stopped with pending tool calls",
            Self::MissingFinalAssistantAfterToolResult => {
                "Agent turn stopped after tool use without a final assistant answer"
            }
            Self::EmptyFinalAssistant => "Agent turn stopped without a final assistant answer",
        }
    }
}

pub fn assess_agent_turn_completion(
    outcome: &AgentTurnOutcome,
    task_assessment: AgentHarnessSuccessAssessment,
    _mode: AgentHarnessMode,
) -> AgentHarnessSuccessAssessment {
    if let Some(reason) = assess_tool_protocol(&outcome.tool_protocol) {
        return AgentHarnessSuccessAssessment {
            state: "incomplete",
            error: Some(reason.message().to_string()),
            task_counts: task_assessment.task_counts,
            turn_incomplete_reason: Some(reason.as_str()),
            tool_protocol: outcome.tool_protocol,
            recent_tools: outcome.recent_tools.clone(),
        };
    }

    if outcome.final_response.trim().is_empty() {
        return AgentHarnessSuccessAssessment {
            state: "incomplete",
            error: Some(
                AgentTurnIncompleteReason::EmptyFinalAssistant
                    .message()
                    .to_string(),
            ),
            task_counts: task_assessment.task_counts,
            turn_incomplete_reason: Some(AgentTurnIncompleteReason::EmptyFinalAssistant.as_str()),
            tool_protocol: outcome.tool_protocol,
            recent_tools: outcome.recent_tools.clone(),
        };
    }

    AgentHarnessSuccessAssessment {
        tool_protocol: outcome.tool_protocol,
        recent_tools: outcome.recent_tools.clone(),
        ..task_assessment
    }
}

fn assess_tool_protocol(protocol: &AgentToolProtocolSummary) -> Option<AgentTurnIncompleteReason> {
    if protocol.pending_tool_call_count > 0 {
        return Some(AgentTurnIncompleteReason::PendingToolCalls);
    }

    if protocol.tool_call_count > 0 && !protocol.final_assistant_after_last_tool_result {
        return Some(AgentTurnIncompleteReason::MissingFinalAssistantAfterToolResult);
    }

    None
}

#[cfg(test)]
mod tests {
    use crate::commands::ai_runtime_harness::{
        AgentHarnessMode, AgentHarnessSuccessAssessment, AgentHarnessTaskLedgerCounts,
    };

    use super::*;

    fn succeeded_task_assessment() -> AgentHarnessSuccessAssessment {
        AgentHarnessSuccessAssessment {
            state: "succeeded",
            error: None,
            task_counts: AgentHarnessTaskLedgerCounts::default(),
            turn_incomplete_reason: None,
            tool_protocol: AgentToolProtocolSummary::default(),
            recent_tools: Vec::new(),
        }
    }

    #[test]
    fn pending_tool_calls_make_turn_incomplete_without_task_ledger() {
        let outcome = AgentTurnOutcome {
            final_response: String::new(),
            stop_reason: crate::agents::AgentTurnStopReason::FinalAssistant,
            recent_tools: Vec::new(),
            tool_protocol: AgentToolProtocolSummary {
                tool_call_count: 1,
                tool_result_count: 0,
                pending_tool_call_count: 1,
                final_assistant_after_last_tool_result: false,
            },
        };

        let assessment = assess_agent_turn_completion(
            &outcome,
            succeeded_task_assessment(),
            AgentHarnessMode::ToolRun,
        );

        assert!(assessment.incomplete());
        assert_eq!(
            assessment.turn_incomplete_reason,
            Some("tool_calls_pending")
        );
    }

    #[test]
    fn missing_final_answer_after_tool_result_is_incomplete() {
        let outcome = AgentTurnOutcome {
            final_response: String::new(),
            stop_reason: crate::agents::AgentTurnStopReason::FinalAssistant,
            recent_tools: Vec::new(),
            tool_protocol: AgentToolProtocolSummary {
                tool_call_count: 1,
                tool_result_count: 1,
                pending_tool_call_count: 0,
                final_assistant_after_last_tool_result: false,
            },
        };

        let assessment = assess_agent_turn_completion(
            &outcome,
            succeeded_task_assessment(),
            AgentHarnessMode::ToolRun,
        );

        assert!(assessment.incomplete());
        assert_eq!(
            assessment.turn_incomplete_reason,
            Some("stopped_after_tool_result_without_final_answer")
        );
    }

    #[test]
    fn empty_direct_response_is_incomplete() {
        let outcome = AgentTurnOutcome::direct(String::new());

        let assessment = assess_agent_turn_completion(
            &outcome,
            succeeded_task_assessment(),
            AgentHarnessMode::Direct,
        );

        assert!(assessment.incomplete());
        assert_eq!(
            assessment.turn_incomplete_reason,
            Some("empty_final_assistant")
        );
    }
}
