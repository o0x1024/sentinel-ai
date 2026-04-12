use std::collections::HashMap;
use std::sync::Mutex;

use once_cell::sync::Lazy;

use super::types::ToolCallRecord;

static EXECUTION_TOOL_TRACES: Lazy<Mutex<HashMap<String, Vec<ToolCallRecord>>>> =
    Lazy::new(|| Mutex::new(HashMap::new()));

pub fn clear_execution_tool_trace(execution_id: &str) {
    let Ok(mut traces) = EXECUTION_TOOL_TRACES.lock() else {
        return;
    };
    traces.remove(execution_id);
}

pub fn append_execution_tool_trace(execution_id: &str, record: ToolCallRecord) {
    let Ok(mut traces) = EXECUTION_TOOL_TRACES.lock() else {
        return;
    };
    traces
        .entry(execution_id.to_string())
        .or_default()
        .push(record);
}

pub fn take_execution_tool_trace(execution_id: &str) -> Vec<ToolCallRecord> {
    let Ok(mut traces) = EXECUTION_TOOL_TRACES.lock() else {
        return Vec::new();
    };
    traces.remove(execution_id).unwrap_or_default()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn appends_and_takes_execution_tool_trace() {
        let execution_id = "sar-test-tool-trace";
        clear_execution_tool_trace(execution_id);
        append_execution_tool_trace(
            execution_id,
            ToolCallRecord {
                id: "tc-1".to_string(),
                name: "lookup".to_string(),
                arguments: r#"{"q":"id"}"#.to_string(),
                result: Some(r#"{"ok":true}"#.to_string()),
                success: true,
                sequence: 0,
                started_at_ms: 1,
                completed_at_ms: 2,
                duration_ms: 1,
            },
        );

        let traces = take_execution_tool_trace(execution_id);
        assert_eq!(traces.len(), 1);
        assert_eq!(traces[0].name, "lookup");
        assert!(take_execution_tool_trace(execution_id).is_empty());
    }
}
