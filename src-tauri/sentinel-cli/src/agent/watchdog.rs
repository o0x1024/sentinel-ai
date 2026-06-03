use crate::runtime::ChallengeRunState;
use serde::Serialize;

const STALL_HINT_THRESHOLD: usize = 3;
const STALL_ABORT_THRESHOLD: usize = 5;
const REPEAT_HINT_THRESHOLD: usize = 3;
const REPEAT_ABORT_THRESHOLD: usize = 5;

#[derive(Debug, Clone, Serialize)]
pub struct WatchdogEvaluation {
    pub stall_count: usize,
    pub repeat_count: usize,
    pub last_tool_signature: Option<String>,
    pub last_result_signature: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum WatchdogAction {
    Continue,
    RequestHint { reason: String },
    InjectFeedback { reason: String },
    Abort { reason: String },
}

pub struct WatchdogInput<'a> {
    pub previous_tool_calls: usize,
    pub current_tool_calls: usize,
    pub previous_flag_count: usize,
    pub current_flag_count: usize,
    pub latest_tool_signature: Option<&'a str>,
    pub latest_result_signature: Option<&'a str>,
    pub hint_allowed: bool,
    pub hint_used: bool,
}

pub fn evaluate_watchdog(
    run_state: &mut ChallengeRunState,
    input: WatchdogInput<'_>,
) -> (WatchdogEvaluation, WatchdogAction) {
    let new_flags_found = input.current_flag_count > input.previous_flag_count;
    let new_tool_calls = input.current_tool_calls > input.previous_tool_calls;
    let result_changed = input
        .latest_result_signature
        .map(|signature| run_state.watchdog_last_result_signature.as_deref() != Some(signature))
        .unwrap_or(false);

    if new_flags_found || result_changed {
        run_state.watchdog_stall_count = 0;
    } else {
        run_state.watchdog_stall_count += 1;
    }

    let latest_tool_signature = input.latest_tool_signature.map(ToOwned::to_owned);
    if let Some(signature) = latest_tool_signature.clone() {
        if run_state.watchdog_last_tool_signature.as_deref() == Some(signature.as_str())
            && !new_flags_found
        {
            run_state.watchdog_repeat_count += 1;
        } else {
            run_state.watchdog_repeat_count = 0;
        }
        run_state.watchdog_last_tool_signature = Some(signature);
    }
    if let Some(result_signature) = input.latest_result_signature.map(ToOwned::to_owned) {
        run_state.watchdog_last_result_signature = Some(result_signature);
    }

    let snapshot = WatchdogEvaluation {
        stall_count: run_state.watchdog_stall_count,
        repeat_count: run_state.watchdog_repeat_count,
        last_tool_signature: run_state.watchdog_last_tool_signature.clone(),
        last_result_signature: run_state.watchdog_last_result_signature.clone(),
    };

    let action = if !input.hint_used
        && input.hint_allowed
        && (run_state.watchdog_stall_count >= STALL_HINT_THRESHOLD
            || run_state.watchdog_repeat_count >= REPEAT_HINT_THRESHOLD)
    {
        run_state.watchdog_interventions += 1;
        run_state.watchdog_stall_count = 0;
        run_state.watchdog_repeat_count = 0;
        WatchdogAction::RequestHint {
            reason: "watchdog detected repeated attempts without meaningful progress".to_string(),
        }
    } else if input.hint_used
        && (run_state.watchdog_stall_count >= STALL_ABORT_THRESHOLD
            || run_state.watchdog_repeat_count >= REPEAT_ABORT_THRESHOLD)
    {
        run_state.watchdog_interventions += 1;
        WatchdogAction::Abort {
            reason: "watchdog detected sustained loop after hint with no new progress".to_string(),
        }
    } else if run_state.watchdog_repeat_count > 0
        && run_state.watchdog_repeat_count % 2 == 0
        && !input.hint_used
    {
        WatchdogAction::InjectFeedback {
            reason:
                "watchdog noticed repeated tools or unchanged results; try a different endpoint, method, parameter, or command"
                    .to_string(),
        }
    } else if !new_tool_calls && !new_flags_found {
        WatchdogAction::InjectFeedback {
            reason:
                "watchdog saw no new tool activity or flags in the last step; change strategy instead of repeating the same path"
                    .to_string(),
        }
    } else {
        WatchdogAction::Continue
    };

    (snapshot, action)
}
