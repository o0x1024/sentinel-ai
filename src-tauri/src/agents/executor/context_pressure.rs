use serde_json::json;
use tauri::{AppHandle, Emitter};

use sentinel_llm::ChatMessage;

use crate::agents::context_engineering::{
    estimate_message_tokens, estimate_tokens, ContextBudgetAnalysis, ContextBudgetAnalyzer,
    ContextPressure, SYSTEM_MESSAGE_OVERHEAD_TOKENS,
};
use crate::commands::ai_runtime_harness::append_agent_harness_event;

#[derive(Debug, Clone, Copy)]
pub(super) struct RequestContextPressure {
    pub analysis: ContextBudgetAnalysis,
    pub should_compact: bool,
    pub should_block: bool,
    pub system_tokens: usize,
    pub task_tokens: usize,
    pub history_tokens: usize,
}

pub(super) fn analyze_request_context_pressure(
    analyzer: &ContextBudgetAnalyzer,
    system_prompt: Option<&str>,
    task: &str,
    history_messages: &[ChatMessage],
) -> RequestContextPressure {
    let system_tokens = system_prompt
        .map(|prompt| estimate_tokens(prompt) + SYSTEM_MESSAGE_OVERHEAD_TOKENS)
        .unwrap_or(0);
    let task_tokens = estimate_tokens(task);
    let history_tokens: usize = history_messages.iter().map(estimate_message_tokens).sum();
    let used_tokens = system_tokens + task_tokens + history_tokens;
    let analysis = analyzer.analyze(used_tokens);
    let should_compact = matches!(
        analysis.pressure,
        ContextPressure::AutoCompact | ContextPressure::Blocking
    );
    let should_block = matches!(analysis.pressure, ContextPressure::Blocking);

    RequestContextPressure {
        analysis,
        should_compact,
        should_block,
        system_tokens,
        task_tokens,
        history_tokens,
    }
}

pub(super) fn emit_request_context_pressure(
    app_handle: &AppHandle,
    execution_id: &str,
    generation: Option<u64>,
    phase: &str,
    analyzer: &ContextBudgetAnalyzer,
    system_prompt: Option<&str>,
    task: &str,
    history_messages: &[ChatMessage],
) -> RequestContextPressure {
    let pressure =
        analyze_request_context_pressure(analyzer, system_prompt, task, history_messages);

    let _ = app_handle.emit(
        "agent:context_pressure",
        &json!({
            "execution_id": execution_id,
            "generation": generation,
            "phase": phase,
            "used_tokens": pressure.analysis.used_tokens,
            "remaining_tokens": pressure.analysis.remaining_tokens,
            "usage_percentage": pressure.analysis.usage_percentage,
            "context_pressure": format!("{:?}", pressure.analysis.pressure),
            "should_compact": pressure.should_compact,
            "should_block": pressure.should_block,
            "system_prompt_tokens": pressure.system_tokens,
            "task_tokens": pressure.task_tokens,
            "history_tokens": pressure.history_tokens,
            "history_count": history_messages.len(),
            "max_context_tokens": analyzer.max_context_tokens,
            "effective_context_tokens": analyzer.effective_context_tokens,
            "warning_threshold_tokens": analyzer.warning_threshold_tokens,
            "auto_compact_threshold_tokens": analyzer.auto_compact_threshold_tokens,
            "blocking_threshold_tokens": analyzer.blocking_threshold_tokens,
            "output_reserve_tokens": analyzer.output_reserve_tokens,
        }),
    );

    pressure
}

pub(super) async fn append_request_context_pressure_harness_event(
    app_handle: &AppHandle,
    harness_run_id: Option<&str>,
    execution_id: &str,
    generation: Option<u64>,
    event_type: &str,
    phase: &str,
    analyzer: &ContextBudgetAnalyzer,
    pressure: &RequestContextPressure,
    history_count: usize,
) {
    let (Some(run_id), Some(generation)) = (harness_run_id, generation) else {
        return;
    };
    append_agent_harness_event(
        app_handle,
        run_id,
        execution_id,
        generation,
        event_type,
        Some(build_request_context_pressure_payload(
            phase,
            analyzer,
            pressure,
            history_count,
        )),
    )
    .await;
}

fn build_request_context_pressure_payload(
    phase: &str,
    analyzer: &ContextBudgetAnalyzer,
    pressure: &RequestContextPressure,
    history_count: usize,
) -> serde_json::Value {
    json!({
        "phase": phase,
        "used_tokens": pressure.analysis.used_tokens,
        "remaining_tokens": pressure.analysis.remaining_tokens,
        "usage_percentage": pressure.analysis.usage_percentage,
        "context_pressure": format!("{:?}", pressure.analysis.pressure),
        "should_compact": pressure.should_compact,
        "should_block": pressure.should_block,
        "system_prompt_tokens": pressure.system_tokens,
        "task_tokens": pressure.task_tokens,
        "history_tokens": pressure.history_tokens,
        "history_count": history_count,
        "max_context_tokens": analyzer.max_context_tokens,
        "effective_context_tokens": analyzer.effective_context_tokens,
        "warning_threshold_tokens": analyzer.warning_threshold_tokens,
        "auto_compact_threshold_tokens": analyzer.auto_compact_threshold_tokens,
        "blocking_threshold_tokens": analyzer.blocking_threshold_tokens,
        "output_reserve_tokens": analyzer.output_reserve_tokens,
    })
}

#[cfg(test)]
mod tests {
    use sentinel_llm::ChatMessage;

    use super::{analyze_request_context_pressure, build_request_context_pressure_payload};
    use crate::agents::context_engineering::{ContextBudgetAnalyzer, ContextPressure};

    #[test]
    fn marks_auto_compact_pressure_for_large_request_context() {
        let analyzer = ContextBudgetAnalyzer::new(8_192);
        let large_history = vec![ChatMessage::user("x".repeat(20_000))];

        let pressure =
            analyze_request_context_pressure(&analyzer, Some("system"), "task", &large_history);

        assert!(pressure.should_compact);
        assert!(matches!(
            pressure.analysis.pressure,
            ContextPressure::AutoCompact | ContextPressure::Blocking
        ));
    }

    #[test]
    fn low_pressure_does_not_request_compaction() {
        let analyzer = ContextBudgetAnalyzer::new(128_000);
        let history = vec![ChatMessage::user("short")];

        let pressure =
            analyze_request_context_pressure(&analyzer, Some("system"), "task", &history);

        assert!(!pressure.should_compact);
        assert!(!pressure.should_block);
        assert_eq!(pressure.analysis.pressure, ContextPressure::Low);
    }

    #[test]
    fn harness_payload_includes_thresholds_and_phase() {
        let analyzer = ContextBudgetAnalyzer::new(128_000);
        let history = vec![ChatMessage::user("short")];
        let pressure =
            analyze_request_context_pressure(&analyzer, Some("system"), "task", &history);

        let payload =
            build_request_context_pressure_payload("pre_stream_request", &analyzer, &pressure, 7);

        assert_eq!(payload["phase"], "pre_stream_request");
        assert_eq!(payload["history_count"], 7);
        assert_eq!(
            payload["effective_context_tokens"],
            analyzer.effective_context_tokens
        );
        assert_eq!(
            payload["auto_compact_threshold_tokens"],
            analyzer.auto_compact_threshold_tokens
        );
        assert_eq!(payload["context_pressure"], "Low");
    }
}
